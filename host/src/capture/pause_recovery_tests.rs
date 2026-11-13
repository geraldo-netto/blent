//! T1415: failed pause control retires native work without killing its supervisor.
use super::*;
use blent::display_quiescence::{channel, Control, Fence};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};

async fn stream_recovers(marker: &str) {
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let original = stream.helper();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join(marker), b"T1415 private pause-control failure").unwrap();
    stream.lock(true);
    until(|| stream.task.as_ref().unwrap().is_finished() || stream.headers.current().is_none())
        .await;
    assert!(
        !stream.task.as_ref().unwrap().is_finished(),
        "T1415 failed pause killed capture supervisor"
    );
    assert!(!first.generation.load(Ordering::Acquire));
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        stream.helper(),
        original,
        "T1415 replaced helper while paused"
    );
    assert!(stream.headers.current().is_none());
    std::fs::remove_file(root.join(marker)).unwrap();
    stream.lock(false);
    let fresh = stream.fresh(&first).await;
    assert!(fresh.is_idr && fresh.codec_config.is_some());
    assert_ne!(stream.helper(), original);
    stream.stop().await;
}

#[tokio::test]
async fn t1415_invalid_pause_ack_recovers_only_on_renewed_demand() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_invalid_pause_ack_recovers_only_on_renewed_demand") { return; }
    stream_recovers("foreign-ack").await;
}

#[tokio::test]
async fn t1415_pause_control_eof_recovers_only_on_renewed_demand() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_pause_control_eof_recovers_only_on_renewed_demand") { return; }
    stream_recovers("close-pause").await;
}

#[tokio::test]
async fn t1415_encoder_replacement_recovers_after_retained_helper_pause_eof() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_encoder_replacement_recovers_after_retained_helper_pause_eof") { return; }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let original = stream.helper();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("close-pause"), b"fail retained helper pause").unwrap();
    stream
        .settings
        .send_modify(|settings| settings.bitrate += 1000);
    let fresh = stream.fresh(&first).await;
    assert!(fresh.is_idr && fresh.codec_config.is_some());
    assert_ne!(stream.helper(), original);
    stream.stop().await;
}

struct HeldCleanup {
    entered: Arc<AtomicBool>,
    release: std::sync::Mutex<mpsc::Receiver<()>>,
}
impl Drop for HeldCleanup {
    fn drop(&mut self) {
        self.entered.store(true, Ordering::Release);
        let _ = self.release.get_mut().unwrap().recv();
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn bind_pause(manager: &mut CaptureManager) -> (Control, Fence) {
    let (control, mut participants) = channel(1).unwrap();
    let participant = participants.pop().unwrap();
    manager.bind_display_work(Arc::new(
        blent::display_work::View::new(None, None, false).with_gate(participant.source()),
    ));
    manager.bind_quiescence(participant);
    let fence = control.pause();
    (control, fence)
}

async fn held_failure() -> (
    CaptureManager,
    blent_config::retirement::Completion,
    Arc<AtomicBool>,
    Release,
) {
    let mut manager = streaming_manager();
    manager.bind_display_work(Arc::new(blent::display_work::View::new(None, None, false)));
    manager.start_helper().await.unwrap();
    manager.helper.set_capture_active(true).await.unwrap();
    let (release, hold) = mpsc::channel();
    let entered = Arc::new(AtomicBool::new(false));
    let child = manager.helper.child.as_mut().unwrap();
    let original = child.completion();
    child.keep_alive(HeldCleanup {
        entered: entered.clone(),
        release: std::sync::Mutex::new(hold),
    });
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("foreign-ack"), b"fail original pause").unwrap();
    (manager, original, entered, Release(release))
}

fn spawn_pause(
    mut manager: CaptureManager,
    stopped: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<(CaptureManager, anyhow::Result<()>)> {
    tokio::spawn(async move {
        let result = manager.pause_requested(&stopped).await;
        (manager, result)
    })
}

#[tokio::test]
async fn t1415_failed_pause_waits_for_original_cleanup_before_quiescence() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_failed_pause_waits_for_original_cleanup_before_quiescence") { return; }
    let (mut manager, original, entered, release) = held_failure().await;
    let (control, fence) = bind_pause(&mut manager);
    let (_stop, stopped) = watch::channel(false);
    let paused = spawn_pause(manager, stopped);
    until(|| entered.load(Ordering::Acquire) || paused.is_finished()).await;
    let returned_early = paused.is_finished();
    assert!(
        !fence.complete(),
        "T1415 pause receipt preceded original cleanup"
    );
    assert!(!control.resume(&fence));
    assert!(!original.is_complete());
    drop(release);
    let (mut manager, result) = paused.await.unwrap();
    manager.shutdown().await;
    assert!(
        !returned_early,
        "T1415 pause failure escaped native retirement"
    );
    result.unwrap();
    assert!(original.is_complete());
    assert!(fence.complete());
    assert!(control.resume(&fence));
}

#[tokio::test]
async fn t1415_cancelled_failed_pause_retains_original_cleanup_for_retry() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_cancelled_failed_pause_retains_original_cleanup_for_retry") { return; }
    let (mut manager, original, entered, release) = held_failure().await;
    let (control, fence) = bind_pause(&mut manager);
    let (stop, stopped) = watch::channel(false);
    let paused = spawn_pause(manager, stopped);
    until(|| entered.load(Ordering::Acquire)).await;
    stop.send_replace(true);
    let (manager, result) = paused.await.unwrap();
    result.unwrap();
    assert!(!original.is_complete());
    assert!(!fence.complete());
    let (_continue, running) = watch::channel(false);
    let retry = spawn_pause(manager, running);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !retry.is_finished(),
        "T1415 retry forgot original helper cleanup"
    );
    assert!(!control.resume(&fence));
    drop(release);
    let (mut manager, result) = retry.await.unwrap();
    result.unwrap();
    assert!(original.is_complete());
    assert!(control.resume(&fence));
    manager.shutdown().await;
}

#[tokio::test]
async fn t1415_failed_pause_retains_cancelled_reader_generation_until_native_exit() {
    if ownership_tests::isolated("capture::activity_tests::pause_recovery::t1415_failed_pause_retains_cancelled_reader_generation_until_native_exit") { return; }
    let mut manager = streaming_manager();
    manager.bind_display_work(Arc::new(blent::display_work::View::new(None, None, false)));
    manager.start_helper().await.unwrap();
    manager.helper.set_capture_active(true).await.unwrap();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("foreign-ack"), b"fail original pause").unwrap();
    let (original, release) = retain_reader(&mut manager).await;
    let (control, fence) = bind_pause(&mut manager);
    let (_stop, stopped) = watch::channel(false);
    let paused = spawn_pause(manager, stopped);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!paused.is_finished());
    assert!(!original.is_complete());
    assert!(!fence.complete());
    drop(release);
    let (mut manager, result) = paused.await.unwrap();
    result.unwrap();
    assert!(original.is_complete());
    assert!(control.resume(&fence));
    manager.shutdown().await;
}

async fn retain_reader(
    manager: &mut CaptureManager,
) -> (blent_config::retirement::Completion, Release) {
    let (release, held) = mpsc::channel();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let native = blent::owned_task::Task::spawn_blocking(move || {
        entered.send(()).unwrap();
        let _ = held.recv();
        Ok(())
    })
    .unwrap();
    ready.await.unwrap();
    let original = native.completion();
    let scope = manager.encoder.begin_generation().unwrap();
    scope.retain(original.clone());
    let child =
        blent_config::commands::OwnedChild::spawn(&mut tokio::process::Command::new("/bin/true"))
            .unwrap();
    manager.encoder.adopt(child, scope);
    native.abort();
    drop(native);
    (original, Release(release))
}
