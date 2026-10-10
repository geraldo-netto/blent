use super::*;

#[test]
fn t931_usb_selection_rejects_ambiguous_or_invalid_settings() {
    for value in [
        serde_json::json!({"usb_peer":"a".repeat(64),"serial":"tablet"}),
        serde_json::json!({"usb_peer":""}),
        serde_json::json!({"usb_peer":"A".repeat(64)}),
        serde_json::json!({"usb_peer":"a".repeat(65)}),
    ] {
        let profile: CameraProfile = serde_json::from_value(value).unwrap();
        assert!(
            profile.validate().is_err(),
            "T931 ambiguous/invalid target accepted"
        );
    }
}

#[test]
fn t931_target_selection_and_saved_legacy_defaults_are_explicit() {
    use connection::Target;
    let mut profile: CameraProfile = serde_json::from_str("{}").unwrap();
    assert_eq!(profile.target().unwrap(), Target::Adb(None));
    let legacy = serde_json::to_value(&profile).unwrap();
    assert!(legacy.get("usb_peer").is_none());
    profile.serial = Some("tablet ' ; $()".into());
    assert_eq!(
        profile.target().unwrap(),
        Target::Adb(profile.serial.as_deref())
    );
    profile.serial = None;
    profile.usb_peer = Some("1".repeat(64));
    let Target::Usb(peer) = profile.target().unwrap() else {
        panic!("T931 USB became ADB")
    };
    assert_eq!(peer.as_str(), "1".repeat(64));
    let restored: CameraProfile =
        serde_json::from_value(serde_json::to_value(&profile).unwrap()).unwrap();
    assert_eq!(restored, profile);
    for length in 0..=128 {
        profile.usb_peer = Some("a".repeat(length));
        assert_eq!(profile.validate().is_ok(), length == 64);
    }
    for byte in 0u8..=127 {
        profile.usb_peer = Some(format!("{}{}", "a".repeat(63), char::from(byte)));
        assert_eq!(
            profile.validate().is_ok(),
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        );
    }
}

#[test]
fn t931_wire_projection_preserves_camera_controls_and_host_transforms() {
    use crate::connection::management::{Credential, Key, Message, Profile, Resource, MAX_RECORD};
    let mut profile = CameraProfile::default();
    profile.usb_peer = Some("a".repeat(64));
    profile.rotation = 270;
    profile.mirror = true;
    profile.background = true;
    profile.adaptive_bitrate = false;
    profile.min_bitrate = 20000;
    for lens in [Lens::Front, Lens::Rear] {
        profile.lens = lens;
        let projected = profile.invitation_profile().unwrap();
        assert_eq!(projected.width, 1280);
        assert_eq!(projected.height, 720);
        assert_eq!(projected.fps, 30);
        assert_eq!(projected.bitrate, 3000);
        assert_eq!(projected.min_bitrate, 20000);
        assert_eq!(projected.freshness_ms, 150);
        assert_eq!(projected.lens, u8::from(lens == Lens::Rear));
        assert!(projected.background);
        assert!(!projected.adaptive);
        let offer = Message::Offer(
            Key {
                resource: Resource::Camera,
                request: 1,
                generation: 1,
            },
            Credential::new([1; 32]),
            Profile::Camera(projected),
        );
        let mut wire = [0; MAX_RECORD];
        let length = offer.encode(&mut wire).unwrap();
        assert!(Message::decode(&wire[..length]).unwrap() == offer);
    }
    profile.width = u32::MAX;
    assert!(
        profile.invitation_profile().is_err(),
        "T931 narrowing must follow validation"
    );
}

#[test]
fn t931_credentials_reject_all_invalid_lengths_and_ascii_bytes() {
    for length in 0..=128 {
        assert_eq!(
            connection::validate_credential(&"a".repeat(length)).is_ok(),
            length == 64
        );
    }
    for byte in 0u8..=127 {
        let token = format!("{}{}", "a".repeat(63), char::from(byte));
        assert_eq!(
            connection::validate_credential(&token).is_ok(),
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        );
    }
    assert!(connection::validate_credential(&"é".repeat(32)).is_err());
}

#[cfg(feature = "platform")]
#[test]
fn t931_cli_rejects_mixed_transports_and_keeps_native_selection_explicit() {
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    let peer = "a".repeat(64);
    let Some(Commands::Cameras(options)) =
        Cli::try_parse_from(["blent", "cameras", "--usb-peer", &peer])
            .unwrap()
            .command
    else {
        panic!()
    };
    assert_eq!(options.usb_peer.as_deref(), Some(peer.as_str()));
    assert!(options.serial.is_none());
    assert!(Cli::try_parse_from([
        "blent",
        "cameras",
        "--usb-peer",
        &peer,
        "--serial",
        "tablet"
    ])
    .is_err());
    let Some(Commands::Cameras(options)) =
        Cli::try_parse_from(["blent", "cameras", "--serial", "tablet"])
            .unwrap()
            .command
    else {
        panic!()
    };
    assert_eq!(options.serial.as_deref(), Some("tablet"));
    assert!(options.usb_peer.is_none());
}
