//! T812: terminal worker failures must be observable and require a fresh Start.
use blent_config::audio::{AudioController, AudioOptions, AudioState, Direction};
use blent_config::camera::{CameraBackend, CameraProfile, CameraState};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

fn wait(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "T812 worker failure was not published"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn t812_audio_setup_and_poll_panics_are_reported_and_manual_restart_recovers() {
    for setup in [false, true] {
        let attempts = Arc::new(AtomicUsize::new(0));
        let calls = attempts.clone();
        let controller = blent::audio_control::Controller::new(move |_, mut stop, report| {
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            if first && setup {
                panic!("setup fixture");
            }
            async move {
                if first {
                    panic!("poll fixture");
                }
                report.update(AudioState::Streaming, "ready");
                blent::camera_control::cancelled(&mut stop).await;
                Ok(())
            }
        })
        .unwrap();
        let options = AudioOptions::new(Direction::Microphone);
        controller.start(options.clone()).unwrap();
        wait(|| controller.status().state == AudioState::Failed);
        assert!(controller.status().detail.contains("fixture"));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        controller.start(options).unwrap();
        wait(|| controller.status().state == AudioState::Streaming);
        controller.stop();
        wait(|| controller.status().state == AudioState::Stopped);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        let complete = controller.shutdown();
        wait(|| complete.is_complete());
    }
}

#[test]
fn t812_camera_setup_and_poll_panics_are_reported_and_manual_restart_recovers() {
    for setup in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let attempts = calls.clone();
        let controller = blent::camera_control::Controller::new(move |_, mut stop, report| {
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            if first && setup {
                panic!("setup fixture");
            }
            async move {
                if first {
                    panic!("poll fixture");
                }
                report.update(CameraState::Streaming);
                blent::camera_control::cancelled(&mut stop).await;
                Ok(())
            }
        })
        .unwrap();
        controller.start(CameraProfile::default()).unwrap();
        wait(|| matches!(controller.state(), CameraState::Failed(_)));
        assert!(
            matches!(controller.state(), CameraState::Failed(message) if message.contains("fixture"))
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        controller.start(CameraProfile::default()).unwrap();
        wait(|| controller.state() == CameraState::Streaming);
        controller.stop();
        wait(|| controller.state() == CameraState::Stopped);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        let complete = controller.shutdown();
        wait(|| complete.is_complete());
    }
}

#[test]
fn t812_stop_cleanup_errors_remain_failed_until_explicit_action() {
    let audio = blent::audio_control::Controller::new(|_, mut stop, report| async move {
        report.update(AudioState::Streaming, "ready");
        blent::camera_control::cancelled(&mut stop).await;
        anyhow::bail!("cleanup fixture");
    })
    .unwrap();
    audio.start(AudioOptions::new(Direction::Speakers)).unwrap();
    wait(|| audio.status().state == AudioState::Streaming);
    audio.stop();
    wait(|| audio.status().state == AudioState::Failed);
    assert!(audio.status().detail.contains("cleanup fixture"));
    let done = audio.shutdown();
    wait(|| done.is_complete());
    let camera = blent::camera_control::Controller::new(|_, mut stop, report| async move {
        report.update(CameraState::Streaming);
        blent::camera_control::cancelled(&mut stop).await;
        panic!("cleanup fixture");
    })
    .unwrap();
    camera.start(CameraProfile::default()).unwrap();
    wait(|| camera.state() == CameraState::Streaming);
    camera.stop();
    wait(|| matches!(camera.state(), CameraState::Failed(_)));
    let done = camera.shutdown();
    wait(|| done.is_complete());
}

struct DropPanic;
impl std::future::Future for DropPanic {
    type Output = anyhow::Result<()>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(Ok(()))
    }
}
impl Drop for DropPanic {
    fn drop(&mut self) {
        panic!("destructor fixture");
    }
}

#[test]
fn t812_outer_worker_boundary_reports_destructor_panics() {
    let audio = blent::audio_control::Controller::new(|_, _, _| DropPanic).unwrap();
    audio
        .start(AudioOptions::new(Direction::Microphone))
        .unwrap();
    wait(|| audio.status().state == AudioState::Failed);
    assert!(audio.status().detail.contains("destructor fixture"));
    let done = audio.shutdown();
    wait(|| done.is_complete());
    let camera = blent::camera_control::Controller::new(|_, _, _| DropPanic).unwrap();
    camera.start(CameraProfile::default()).unwrap();
    wait(|| matches!(camera.state(), CameraState::Failed(_)));
    assert!(
        matches!(camera.state(), CameraState::Failed(message) if message.contains("destructor fixture"))
    );
    let done = camera.shutdown();
    wait(|| done.is_complete());
}
