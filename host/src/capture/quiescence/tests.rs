//! T1050: native owners, held completion and original helper ACK drive receipts.
use super::*;
use crate::capture::{activity_tests as fixture, ownership_tests::isolated};
use blent::{
    display_quiescence::{channel, Control, Fence},
    owned_task::Task,
};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
async fn held_native(scope: Option<blent::native_scope::Scope>) -> (Task, Release) {
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let observed = scope.clone();
    let task = Task::spawn_blocking(move || {
        entered.send(()).unwrap();
        let _ = held.recv();
        if let Some(scope) = scope {
            scope.sync_output(
                &mut std::process::Command::new("/bin/true"),
                Duration::from_secs(3),
            )?;
        }
        Ok(())
    })
    .unwrap();
    if let Some(scope) = observed {
        scope.retain(task.completion());
    }
    ready.await.unwrap();
    (task, Release(release))
}
fn bind(manager: &mut CaptureManager) -> Control {
    let (control, mut participants) = channel(1).unwrap();
    let participant = participants.pop().unwrap();
    manager.bind_display_work(Arc::new(
        blent::display_work::View::new(None, None, false).with_gate(participant.source()),
    ));
    manager.bind_quiescence(participant);
    control
}
async fn completed(fence: &Fence) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !fence.complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1050_original_helper_and_encoder_native_cleanup_fence_registration() {
    if isolated("capture::quiescence::tests::t1050_original_helper_and_encoder_native_cleanup_fence_registration") { return; }
    for helper in [false, true] {
        let mut manager = fixture::manager();
        let control = bind(&mut manager);
        let scope = if helper {
            None
        } else {
            let scope = manager.encoder.begin_generation().unwrap();
            let child = blent_config::commands::OwnedChild::spawn(
                &mut tokio::process::Command::new("/bin/true"),
            )
            .unwrap();
            manager.encoder.adopt(child, scope.clone());
            Some(scope)
        };
        let (native, release) = held_native(scope).await;
        let original = native.completion();
        native.abort();
        drop(native);
        if helper {
            manager.helper.pending = Some(original.clone());
        }
        let fence = control.pause();
        let (_stop, stopped) = watch::channel(false);
        let pause = tokio::spawn(async move {
            manager.pause_requested(&stopped).await.unwrap();
            manager
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!original.is_complete());
        assert!(
            !fence.complete(),
            "T1050 logical pause released held native owner"
        );
        assert!(!control.resume(&fence));
        drop(release);
        completed(&fence).await;
        let mut manager = pause.await.unwrap();
        assert!(original.is_complete());
        assert!(control.resume(&fence));
        manager.shutdown().await;
    }
}

#[tokio::test]
async fn t1050_original_helper_pause_ack_precedes_receipt_and_preserves_monitor() {
    if isolated("capture::quiescence::tests::t1050_original_helper_pause_ack_precedes_receipt_and_preserves_monitor") { return; }
    let mut manager = fixture::manager();
    let control = bind(&mut manager);
    manager.start_helper().await.unwrap();
    manager.helper.set_capture_active(true).await.unwrap();
    let pid = manager.helper.child.as_ref().unwrap().id();
    let root = manager.config.helper_path.parent().unwrap().to_owned();
    std::fs::write(root.join("hold"), b"hold original false ACK").unwrap();
    let fence = control.pause();
    let (_stop, stopped) = watch::channel(false);
    let paused = tokio::spawn(async move {
        manager.pause_requested(&stopped).await.unwrap();
        manager
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::fs::read_to_string(root.join("requests"))
            .unwrap_or_default()
            .lines()
            .count()
            < 2
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        !fence.complete(),
        "T1050 helper writer had not acknowledged pause"
    );
    std::fs::write(root.join("release"), b"release original ACK").unwrap();
    completed(&fence).await;
    let mut manager = paused.await.unwrap();
    assert_eq!(manager.helper.child.as_ref().unwrap().id(), pid);
    assert!(control.resume(&fence));
    manager.shutdown().await;
}

#[tokio::test]
async fn t1050_cancelled_observer_never_manufactures_completion_or_affects_sibling() {
    if isolated("capture::quiescence::tests::t1050_cancelled_observer_never_manufactures_completion_or_affects_sibling") { return; }
    let mut manager = fixture::manager();
    let control = bind(&mut manager);
    let (sibling, participants) = channel(1).unwrap();
    let source = participants[0].source();
    let permit = source.begin().unwrap();
    let (native, release) = held_native(None).await;
    let original = native.completion();
    manager.helper.pending = Some(original.clone());
    let fence = control.pause();
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(async move {
        manager.pause_requested(&stopped).await.unwrap();
        drop(manager);
    });
    stop.send_replace(true);
    task.await.unwrap();
    assert!(!fence.complete());
    assert!(!control.resume(&fence));
    assert!(source.permits(&permit));
    assert!(!original.is_complete());
    drop(release);
    native.await.unwrap().unwrap();
    assert!(original.is_complete());
    assert!(source.permits(&permit));
    drop(sibling);
}

#[tokio::test]
async fn t1050_idle_capture_wakes_for_original_pause_without_a_display_or_settings_event() {
    if isolated("capture::quiescence::tests::t1050_idle_capture_wakes_for_original_pause_without_a_display_or_settings_event") { return; }
    let mut manager = fixture::manager();
    let control = bind(&mut manager);
    let (_settings, settings) = watch::channel(fixture::settings(&manager.config));
    let (_display, display) = watch::channel(false);
    let (stop, stopped) = watch::channel(false);
    let (video, _) = crate::video_queue::channel(8, Default::default());
    let task = tokio::spawn(async move {
        manager
            .stream_frames(video.into(), settings, display, stopped)
            .await
            .unwrap();
        manager.shutdown().await;
    });
    tokio::task::yield_now().await;
    let fence = control.pause();
    completed(&fence).await;
    assert!(control.resume(&fence));
    stop.send_replace(true);
    task.await.unwrap();
    assert!(!fence.complete());
}
