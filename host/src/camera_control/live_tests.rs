//! Controller/session integration stays with its production owner.
use super::*;

#[test]
fn t1467_controller_live_updates_keep_original_worker_and_start_identity() {
    use blent_config::camera::{CameraBackend, CameraState};
    let (ready, received) = std::sync::mpsc::channel();
    let owner = crate::camera_control::Controller::new(move |_, mut stop, report| {
        let ready = ready.clone();
        async move {
            report.update(CameraState::Streaming);
            ready.send(report.live.unwrap()).unwrap();
            let _ = stop.changed().await;
            Ok(())
        }
    })
    .unwrap();
    let mut profile = CameraProfile::default();
    assert!(owner.adjust(&profile).is_err());
    owner.start(profile.clone()).unwrap();
    let original = received
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    profile.shadow_strength = 70;
    assert!(owner.adjust(&profile).unwrap().pending);
    let (revision, image) = original.snapshot();
    assert_eq!(image.strength, 70);
    original.applied(revision);
    assert!(!owner.adjust(&profile).unwrap().pending);
    assert!(
        received.try_recv().is_err(),
        "live adjustment admitted a new session"
    );
    owner.stop();
    assert!(owner.adjust(&profile).is_err());
    let fence = owner.shutdown();
    let limit = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !fence.is_complete() {
        assert!(std::time::Instant::now() < limit);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
