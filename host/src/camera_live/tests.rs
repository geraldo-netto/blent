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

fn tuning_caps() -> Capabilities {
    Capabilities {
        minimum: -6,
        maximum: 6,
        numerator: 1,
        denominator: 3,
        center: true,
        noise_mask: 7,
        flicker_mask: 15,
    }
}
#[test]
fn t1468_tablet_receipts_do_not_acknowledge_newer_or_replacement_controls() {
    let mut profile = CameraProfile::default();
    let session = Session::new(profile.clone());
    session.capabilities(tuning_caps());
    let first = session.command().unwrap();
    session.applied(first.revision);
    assert!(session.status().pending);
    session.receipt(first.revision, 0).unwrap();
    assert!(!session.status().pending);
    profile.sensor.exposure = 100;
    profile.bitrate = 2500;
    profile.adaptive_bitrate = false;
    let status = session.update(&profile).unwrap();
    assert!(status.pending);
    assert!(!status.restart_required);
    let next = session.command().unwrap();
    assert_eq!(next.sensor.exposure, 100);
    assert_eq!(next.encoding.ceiling, 2500);
    session.applied(next.revision);
    session.receipt(first.revision, 0).unwrap();
    assert!(session.status().pending);
    session.receipt(next.revision, 2).unwrap();
    assert!(!session.status().pending);
    assert!(session.status().error.unwrap().contains("could not"));
    session.capabilities(tuning_caps());
    assert!(session.status().pending);
    session.receipt(next.revision, 1).unwrap();
    assert!(session.status().error.unwrap().contains("unsupported"));
    let replacement = Session::new(profile);
    assert!(replacement.receipt(next.revision, 0).is_err());
    session.close();
    assert!(session.command().is_err());
}
#[test]
fn t1468_unsupported_and_invalid_receipts_preserve_admitted_state() {
    let mut profile = CameraProfile::default();
    let session = Session::new(profile.clone());
    session.capabilities(tuning_caps());
    profile.sensor.exposure = 250;
    let status = session.update(&profile).unwrap();
    assert!(status.error.is_some());
    assert!(status.capabilities.is_some());
    assert_eq!(session.command().unwrap().sensor.exposure, 0);
    for (revision, status) in [(0, 1), (1, 3), (2, 0), (u64::MAX, 0)] {
        assert!(session.receipt(revision, status).is_err());
    }
    session.receipt(1, 0).unwrap();
    assert!(session.receipt(0, 0).is_err());
}
