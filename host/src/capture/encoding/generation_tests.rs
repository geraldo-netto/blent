//! T1036: generation results never substitute for original native retirement.
use super::*;
use blent_config::retirement::Completion;
use std::{
    sync::{mpsc, Mutex},
    time::Duration,
};

struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
struct Held(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>);
impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.0.send(());
        let _ = self.1.lock().unwrap().recv();
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn completed(proof: &Completion) {
    until(|| proof.is_complete()).await;
}
fn held() -> (Held, mpsc::Receiver<()>, Release) {
    let (entered, ready) = mpsc::channel();
    let (release, held) = mpsc::channel();
    (Held(entered, Mutex::new(held)), ready, Release(release))
}
#[tokio::test]
async fn t1036_closed_generation_waits_for_original_joined_reader_dependency() {
    let mut encoder = EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let mut reader = blent::owned_task::Task::spawn(async { Ok(()) }).unwrap();
    scope.retain(reader.completion());
    let (lease, entered, release) = held();
    reader.keep_alive(lease);
    (&mut reader).await.unwrap().unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    encoder.generation.activate(scope);
    encoder.stop();
    let denied = encoder.begin_generation().is_err();
    assert!(encoder.child.is_none());
    drop(release);
    completed(&reader.completion()).await;
    until(|| encoder.ensure_retired().is_ok()).await;
    drop(encoder.begin_generation().unwrap());
    assert!(
        denied,
        "T1036 joined reader released generation before original dependent destruction"
    );
}
#[tokio::test]
async fn t1036_original_creator_alias_and_actual_native_join_each_fence_replacement() {
    let mut encoder = EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let creator = scope.clone();
    encoder.generation.activate(scope);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let worker = blent::owned_task::Task::spawn_blocking(move || {
        drop(creator);
        entered.send(()).unwrap();
        held.recv().unwrap();
        Ok(())
    })
    .unwrap();
    encoder
        .session_report()
        .unwrap()
        .retain(worker.completion());
    ready.await.unwrap();
    encoder.stop();
    let denied = encoder.begin_generation().is_err();
    drop(release);
    worker.await.unwrap().unwrap();
    until(|| encoder.ensure_retired().is_ok()).await;
    assert!(
        denied,
        "T1036 released original native creator after alias dropped but before actual join"
    );
}
#[tokio::test]
async fn t1036_cancelled_acquisition_alias_prevents_next_generation_even_without_a_child() {
    let mut encoder = EncoderProcess::default();
    let creator = encoder.begin_generation().unwrap();
    let denied = encoder.begin_generation().is_err();
    drop(creator);
    until(|| encoder.ensure_retired().is_ok()).await;
    for _ in 0..64 {
        let scope = encoder.begin_generation().unwrap();
        encoder.generation.activate(scope);
        assert!(encoder.begin_generation().is_err());
        encoder.stop();
        assert!(encoder.ensure_retired().is_ok());
    }
    assert!(
        denied,
        "T1036 reserved startup creator was not observed before exposure"
    );
}
#[tokio::test]
async fn t1036_late_child_remains_owned_after_original_creator_returns() {
    let mut encoder = EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let (start, started) = tokio::sync::oneshot::channel();
    let (release_creator, held_creator) = mpsc::channel();
    let release_creator = Release(release_creator);
    let (lease, entered, release_child) = held();
    let worker = blent::owned_task::Task::spawn_blocking({
        let report = scope.clone();
        move || {
            start.send(()).unwrap();
            held_creator.recv().unwrap();
            let mut command = tokio::process::Command::new("/bin/sleep");
            command.arg("60");
            let mut child = report.spawn(&mut command).unwrap();
            child.keep_alive(lease);
            drop(child);
            Ok(())
        }
    })
    .unwrap();
    scope.retain(worker.completion());
    started.await.unwrap();
    drop(scope);
    let before = encoder.begin_generation().is_err();
    drop(release_creator);
    worker.await.unwrap().unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let during = encoder.begin_generation().is_err();
    drop(release_child);
    until(|| encoder.ensure_retired().is_ok()).await;
    assert!(
        before && during,
        "T1036 canceled creator/native cleanup allowed replacement"
    );
}
#[cfg(not(feature = "inproc-encoder"))]
#[tokio::test]
async fn t1036_canceled_actual_encoder_preflight_keeps_original_native_creation() {
    let config = CaptureConfig {
        encoder: "libx264".into(),
        ..Default::default()
    };
    let mut encoder = EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let (start, started) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let mut acquisition =
        Box::pin(
            encoder.start_using(&config, (64, 64), scope, |report| async move {
                report
                    .blocking(move |creator| {
                        drop(creator);
                        start.send(()).unwrap();
                        held.recv().unwrap();
                    })
                    .await
                    .unwrap();
                false
            }),
        );
    assert!(futures_util::poll!(&mut acquisition).is_pending());
    started.await.unwrap();
    drop(acquisition);
    let denied = encoder.begin_generation().is_err();
    drop(release);
    until(|| encoder.ensure_retired().is_ok()).await;
    assert!(
        denied,
        "T1036 canceled production preflight released original native creator"
    );
}
#[tokio::test]
async fn t1036_helper_monitor_scope_remains_independent_of_encoder_generations() {
    if crate::capture::ownership_tests::isolated("capture::encoding::generation_tests::t1036_helper_monitor_scope_remains_independent_of_encoder_generations") { return; }
    use std::os::unix::fs::PermissionsExt;
    let directory = crate::runtime::runtime_dir().unwrap();
    let path = directory.join("t1036-monitor");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf 'EVDI_CONNECTED card17\\n'\nexec sleep 60\n",
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = CaptureConfig {
        helper_path: path,
        edid_path: Some(directory.join("unused.edid")),
        encoder: "libx264".into(),
        raw_transport: blent_config::raw_frame::RawTransport::Fifo,
        ..Default::default()
    };
    let (root, retired) = Scope::default().group();
    let mut manager = crate::capture::CaptureManager::new(config.clone());
    manager.bind_retirement(Arc::new(root));
    manager.helper.start(&config).await.unwrap();
    let pid = manager.helper.child.as_ref().unwrap().id();
    for _ in 0..16 {
        let scope = manager.encoder.begin_generation().unwrap();
        manager.encoder.generation.activate(scope);
        manager.encoder.stop();
        assert!(manager.encoder.ensure_retired().is_ok());
        assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
        assert!(manager.helper.is_running());
        assert!(!retired.is_complete());
    }
    manager.shutdown().await;
    drop(manager);
    until(|| retired.is_complete()).await;
}

#[cfg(not(feature = "inproc-encoder"))]
#[tokio::test]
async fn t1036_failed_child_creation_keeps_original_native_attempt_cleanup() {
    if crate::capture::ownership_tests::isolated("capture::encoding::generation_tests::t1036_failed_child_creation_keeps_original_native_attempt_cleanup") { return; }
    let config = CaptureConfig {
        encoder: "libx264".into(),
        ..Default::default()
    };
    let mut encoder = EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let (lease, entered, release) = held();
    let mut native = blent::owned_task::Task::spawn(async { Ok(()) }).unwrap();
    native.keep_alive(lease);
    scope.retain(native.completion());
    (&mut native).await.unwrap().unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let failed = encoder.start_with(&config, (64, 64), false, scope, |_| {
        Err(std::io::Error::other("T1036 native spawn fault"))
    });
    assert!(failed.is_err());
    let denied = encoder.begin_generation().is_err();
    drop(release);
    completed(&native.completion()).await;
    assert!(encoder.ensure_retired().is_ok());
    assert!(
        denied,
        "T1036 failed child creation released old native attempt cleanup"
    );
}
#[cfg(feature = "inproc-encoder")]
fn output() -> EncoderOutput {
    let (video, _) = crate::video_queue::channel(8, Default::default());
    EncoderOutput {
        fifo: None,
        raw_socket: None,
        tx: video.into(),
        codec_config: Default::default(),
        latency: crate::latency::LatencyTracker::new(),
        idr_wanted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
}
#[cfg(feature = "inproc-encoder")]
#[tokio::test]
async fn t1036_invalid_sdk_worker_jobs_close_unexposed_generation_before_retry() {
    if crate::capture::ownership_tests::isolated("capture::encoding::generation_tests::t1036_invalid_sdk_worker_jobs_close_unexposed_generation_before_retry") { return; }
    let config = CaptureConfig {
        encoder: "libx264".into(),
        ten_bit: true,
        ..Default::default()
    };
    let mut encoder = EncoderProcess::default();
    for dimension in [0, 1, u32::MAX] {
        let scope = encoder.begin_generation().unwrap();
        encoder.start(&config, (64, 64), scope).await.unwrap();
        assert!(encoder
            .spawn_session(&config, Some((dimension, dimension)), output())
            .await
            .is_err());
        assert!(encoder.child.is_none());
        assert!(encoder.ensure_retired().is_ok());
    }
    let scope = encoder.begin_generation().unwrap();
    encoder.start(&config, (64, 64), scope).await.unwrap();
    assert!(encoder
        .spawn_session(&config, None, output())
        .await
        .is_err());
    encoder.stop();
    assert!(encoder.ensure_retired().is_ok());
}
