use super::*;
#[test]
fn t1467_updates_are_bounded_session_bound_and_acknowledged() {
    let profile = CameraProfile::default();
    let first = Session::new(profile.clone());
    let next = Session::new(profile.clone());
    let mut edited = profile.clone();
    edited.shadow_strength = 70;
    assert!(first.update(&edited).unwrap().pending);
    assert_eq!(next.snapshot().1.strength, 40);
    let (revision, image) = first.snapshot();
    assert_eq!(image.strength, 70);
    first.applied(revision + 1);
    assert!(first.status().pending);
    first.applied(revision);
    assert!(!first.status().pending);
    edited.width = 640;
    assert!(first.update(&edited).unwrap().restart_required);
    edited.serial = Some("different-tablet".into());
    assert!(first.update(&edited).is_err());
    first.close();
    assert!(first.update(&profile).is_err());
    assert!(next.update(&profile).is_ok());
}
#[test]
fn t1467_invalid_and_exhausted_updates_preserve_original_settings() {
    let profile = CameraProfile::default();
    let session = Session::new(profile.clone());
    let mut edited = profile.clone();
    edited.shadow_strength = 101;
    assert!(session.update(&edited).is_err());
    assert_eq!(session.snapshot().1, Image::from_profile(&profile));
    session.state.lock().unwrap().revision = u64::MAX;
    edited.shadow_strength = 70;
    assert!(session.update(&edited).is_err());
    assert_eq!(session.snapshot().1, Image::from_profile(&profile));
}

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
