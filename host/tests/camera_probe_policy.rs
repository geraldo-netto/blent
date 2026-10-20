//! T1202: exercise actual preflight without allowing a native ADB effect.
#[path = "../examples/camera-feedback-probe/policy.rs"]
mod policy;
use blent_config::camera::CameraProfile;

#[test]
fn t1202_explicit_usb_target_cannot_admit_an_adb_probe() {
    let profile = CameraProfile {
        usb_peer: Some("a".repeat(64)),
        ..Default::default()
    };
    assert!(profile.validate().is_ok());
    assert!(
        policy::validate(&profile, 1, 0).is_err(),
        "T1202 USB target admitted to ADB probe"
    );
}

#[test]
fn t1202_stall_limit_is_rejected_during_preflight() {
    assert!(
        policy::validate(&CameraProfile::default(), 1, 1001).is_err(),
        "T1202 invalid stall admitted before native setup"
    );
}

#[test]
fn t1202_current_adb_routes_and_limit_endpoints_remain_supported() {
    for serial in [None, Some("tablet-42".into())] {
        let profile = CameraProfile {
            serial,
            ..Default::default()
        };
        for seconds in [1, 30] {
            for stall in [0, 1000] {
                policy::validate(&profile, seconds, stall).unwrap();
            }
        }
    }
}

#[test]
fn t1202_bounded_invalid_transport_values_cannot_select_adb() {
    for length in 0..=65 {
        let profile = CameraProfile {
            usb_peer: Some("a".repeat(length)),
            ..Default::default()
        };
        assert!(policy::validate(&profile, 1, 0).is_err());
    }
    let mixed = CameraProfile {
        serial: Some("tablet".into()),
        usb_peer: Some("a".repeat(64)),
        ..Default::default()
    };
    assert!(policy::validate(&mixed, 1, 0).is_err());
}

#[test]
fn t1202_invalid_policy_boundaries_reject_without_native_effects() {
    for seconds in [0, 31, u64::MAX] {
        assert!(policy::validate(&CameraProfile::default(), seconds, 0).is_err());
    }
    assert!(policy::validate(&CameraProfile::default(), 1, u64::MAX).is_err());
    for width in [0, 1, u32::MAX] {
        let profile = CameraProfile {
            width,
            ..Default::default()
        };
        assert!(policy::validate(&profile, 1, 0).is_err());
    }
}
