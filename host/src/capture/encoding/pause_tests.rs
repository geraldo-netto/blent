//! T1046: actual original helper/process/task ownership, never physical capture.
use super::*;
use crate::capture::{helper, ownership_tests::isolated, CaptureManager};
use std::os::unix::fs::MetadataExt;
use std::{
    sync::{mpsc, Mutex},
    time::Duration,
};
use tokio::time::timeout;

async fn manager() -> CaptureManager {
    let config = helper::activity_tests::configuration();
    let mut manager = CaptureManager::new(config.clone());
    manager.helper.enable_activity();
    manager.helper.start(&config).await.unwrap();
    manager
}
fn reader(manager: &mut CaptureManager) -> EncoderTask {
    let scope = manager.encoder.begin_generation().unwrap();
    let handle = blent::owned_task::Task::spawn(std::future::pending()).unwrap();
    scope.retain(handle.completion());
    let child = Child::spawn(tokio::process::Command::new("/bin/sleep").arg("60")).unwrap();
    manager.encoder.adopt(child, scope);
    EncoderTask { handle, idle: None }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t1046_quiescence_keeps_original_helper_and_retires_each_native_generation() {
    if isolated("capture::encoding::pause_tests::t1046_quiescence_keeps_original_helper_and_retires_each_native_generation") { return; }
    let mut manager = manager().await;
    let pid = manager.helper.child.as_ref().unwrap().id();
    let fifo = crate::runtime::fifo_path_for(manager.config.instance).unwrap();
    for _ in 0..16 {
        let mut reader = reader(&mut manager);
        manager
            .codec_config
            .publish(Some(crate::media_storage::MediaBytes::from_static(
                b"old-csd",
            )));
        manager
            .idr_wanted
            .store(false, std::sync::atomic::Ordering::Release);
        manager.helper.set_capture_active(true).await.unwrap();
        let before = std::fs::metadata(&fifo).unwrap().ino();
        timeout(
            Duration::from_secs(5),
            manager.quiesce_display(Some(&mut reader)),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            manager.helper.is_running(),
            "T1046 pause removed the retained monitor helper"
        );
        assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
        assert_eq!(manager.helper.card, Some(u32::MAX));
        assert!(!manager.encoder.is_running());
        assert!(reader.handle.is_joined());
        assert!(reader.handle.completion().is_complete());
        assert!(manager.encoder.ensure_retired().is_ok());
        assert!(manager.codec_config.current().is_none());
        assert!(manager
            .idr_wanted
            .load(std::sync::atomic::Ordering::Acquire));
        assert_ne!(std::fs::metadata(&fifo).unwrap().ino(), before);
    }
    manager.shutdown().await;
    assert!(!manager.helper.is_running());
}
struct Held(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>);
impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.0.send(());
        let _ = self.1.lock().unwrap().recv();
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[tokio::test]
async fn t1046_canceled_pause_cannot_rotate_fifo_or_reuse_held_original_reader_cleanup() {
    if isolated("capture::encoding::pause_tests::t1046_canceled_pause_cannot_rotate_fifo_or_reuse_held_original_reader_cleanup") { return; }
    let mut manager = manager().await;
    let pid = manager.helper.child.as_ref().unwrap().id();
    let mut reader = reader(&mut manager);
    let original = reader.handle.completion();
    let (entered, ready) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    reader.handle.keep_alive(Held(entered, Mutex::new(held)));
    let fifo = crate::runtime::fifo_path_for(manager.config.instance).unwrap();
    let before = std::fs::metadata(&fifo).unwrap().ino();
    let pending = timeout(
        Duration::from_millis(50),
        manager.quiesce_display(Some(&mut reader)),
    )
    .await
    .is_err();
    let retained = !original.is_complete() && manager.encoder.ensure_retired().is_err();
    let unchanged = std::fs::metadata(&fifo)
        .ok()
        .is_some_and(|metadata| metadata.ino() == before);
    drop(release);
    assert!(
        pending,
        "T1046 pause returned before original native reader cleanup"
    );
    assert!(retained);
    assert!(
        unchanged,
        "T1046 rotated the FIFO while original cleanup was held"
    );
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    timeout(
        Duration::from_secs(5),
        manager.quiesce_display(Some(&mut reader)),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(original.is_complete());
    assert!(manager.encoder.begin_generation().is_ok());
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    manager.shutdown().await;
}

fn requests() -> Vec<String> {
    std::fs::read_to_string(crate::runtime::runtime_dir().unwrap().join("requests"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
#[tokio::test]
async fn t1046_canceled_ack_retries_original_request_and_retains_original_monitor() {
    if isolated("capture::encoding::pause_tests::t1046_canceled_ack_retries_original_request_and_retains_original_monitor") { return; }
    let mut manager = manager().await;
    let pid = manager.helper.child.as_ref().unwrap().id();
    manager.helper.set_capture_active(true).await.unwrap();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("hold"), "hold").unwrap();
    let mut reader = reader(&mut manager);
    assert!(timeout(
        Duration::from_millis(50),
        manager.quiesce_display(Some(&mut reader))
    )
    .await
    .is_err());
    until(|| requests().len() == 2).await;
    assert_eq!(requests(), ["000000000000000101", "000000000000000200"]);
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    std::fs::write(root.join("release"), "release").unwrap();
    timeout(
        Duration::from_secs(5),
        manager.quiesce_display(Some(&mut reader)),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        requests().len(),
        2,
        "T1046 retry replaced the original pending pause"
    );
    let resumed = manager.helper.set_capture_active(true).await.unwrap();
    assert_eq!(resumed.sequence(), 3);
    assert!(resumed.active());
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    manager.shutdown().await;
}
#[tokio::test]
async fn t1046_pause_waits_for_original_canceled_startup_creator_native_join() {
    if isolated("capture::encoding::pause_tests::t1046_pause_waits_for_original_canceled_startup_creator_native_join") { return; }
    let mut manager = manager().await;
    let pid = manager.helper.child.as_ref().unwrap().id();
    let scope = manager.encoder.begin_generation().unwrap();
    let creator = scope.clone();
    manager.encoder.generation.activate(scope);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let work = tokio::spawn(async move {
        creator
            .blocking(move |scope| {
                drop(scope);
                entered.send(()).unwrap();
                held.recv().unwrap();
            })
            .await
    });
    ready.await.unwrap();
    work.abort();
    assert!(work.await.unwrap_err().is_cancelled());
    let pending = timeout(Duration::from_millis(50), manager.quiesce_display(None))
        .await
        .is_err();
    let denied = manager.encoder.begin_generation().is_err();
    drop(release);
    timeout(Duration::from_secs(5), manager.quiesce_display(None))
        .await
        .unwrap()
        .unwrap();
    assert!(
        pending,
        "T1046 parent cancellation falsely completed native startup"
    );
    assert!(denied);
    assert!(manager.encoder.begin_generation().is_ok());
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    manager.shutdown().await;
}
#[tokio::test]
async fn t1046_foreign_ack_denies_quiescence_without_claiming_completion_or_replacing_helper() {
    if isolated("capture::encoding::pause_tests::t1046_foreign_ack_denies_quiescence_without_claiming_completion_or_replacing_helper") { return; }
    std::env::set_var("BLENT_T1035_MODE", "foreign");
    let mut manager = manager().await;
    let pid = manager.helper.child.as_ref().unwrap().id();
    let fifo = crate::runtime::fifo_path_for(manager.config.instance).unwrap();
    let before = std::fs::metadata(&fifo).unwrap().ino();
    let mut reader = reader(&mut manager);
    assert!(manager
        .quiesce_display(Some(&mut reader))
        .await
        .unwrap_err()
        .to_string()
        .contains("foreign capture acknowledgment"));
    assert!(manager.encoder.ensure_retired().is_ok());
    assert!(reader.handle.completion().is_complete());
    assert_eq!(std::fs::metadata(&fifo).unwrap().ino(), before);
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    assert!(manager.helper.set_capture_active(true).await.is_err());
    manager.shutdown().await;
}
#[tokio::test]
async fn t1046_unstarted_pause_is_safe_and_uncontrolled_helper_remains_explicitly_unsupported() {
    if isolated("capture::encoding::pause_tests::t1046_unstarted_pause_is_safe_and_uncontrolled_helper_remains_explicitly_unsupported") { return; }
    let config = helper::activity_tests::configuration();
    let mut manager = CaptureManager::new(config.clone());
    manager.quiesce_display(None).await.unwrap();
    assert!(manager.codec_config.current().is_none());
    assert!(manager
        .idr_wanted
        .load(std::sync::atomic::Ordering::Acquire));
    manager.helper.start(&config).await.unwrap();
    assert!(manager
        .quiesce_display(None)
        .await
        .unwrap_err()
        .to_string()
        .contains("capture activity control unavailable"));
    assert!(manager.helper.is_running());
    manager.shutdown().await;
}

#[tokio::test]
async fn t1046_reader_join_keeps_progress_after_stop_grace_without_canceling_transaction() {
    if isolated("capture::encoding::pause_tests::t1046_reader_join_keeps_progress_after_stop_grace_without_canceling_transaction") { return; }
    std::env::set_var("BLENT_ENCODER_STOP_MS", "1");
    let mut manager = manager().await;
    let scope = manager.encoder.begin_generation().unwrap();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let handle = blent::owned_task::Task::spawn_blocking(move || {
        entered.send(()).unwrap();
        held.recv().unwrap();
        Ok(())
    })
    .unwrap();
    scope.retain(handle.completion());
    manager.encoder.generation.activate(scope);
    let mut reader = EncoderTask { handle, idle: None };
    ready.await.unwrap();
    let mut pause = Box::pin(manager.quiesce_display(Some(&mut reader)));
    assert!(timeout(Duration::from_millis(50), &mut pause)
        .await
        .is_err());
    drop(release);
    timeout(Duration::from_secs(5), &mut pause)
        .await
        .expect("T1046 grace expiry abandoned original reader join progress")
        .unwrap();
    drop(pause);
    assert!(reader.handle.is_joined());
    assert!(manager.encoder.ensure_retired().is_ok());
    manager.shutdown().await;
}
