use super::*;
use crate::{
    audio::{AudioSettings, Processing},
    connection::management::Profile,
};

#[test]
fn t977_transport_selection_is_explicit_and_independent_per_direction() {
    for direction in [Direction::Microphone, Direction::Speakers] {
        let mut options = AudioOptions::new(direction);
        assert_eq!(options.target().unwrap(), Target::Adb(None));
        options.serial = Some("exact tablet; no shell interpolation".into());
        assert_eq!(
            options.target().unwrap(),
            Target::Adb(options.serial.as_deref())
        );
        options.usb_peer = Some("a".repeat(64));
        assert!(options.validate().is_err());
        options.serial = None;
        let Target::Usb(peer) = options.target().unwrap() else {
            panic!("USB must stay USB")
        };
        assert_eq!(peer.as_str(), "a".repeat(64));
        assert!(options.validate().is_ok());
        assert_eq!(
            AudioOptions::new(direction).target().unwrap(),
            Target::Adb(None)
        );
    }
}

#[test]
fn t977_all_peer_lengths_and_ascii_mutations_fail_closed() {
    let mut options = AudioOptions::new(Direction::Speakers);
    for length in 0..=128 {
        options.usb_peer = Some("a".repeat(length));
        assert_eq!(options.validate().is_ok(), length == 64, "length {length}");
    }
    for byte in 0..=127u8 {
        let mut peer = "a".repeat(64).into_bytes();
        peer[31] = byte;
        options.usb_peer = Some(String::from_utf8(peer).unwrap());
        assert_eq!(
            options.validate().is_ok(),
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        );
    }
    for peer in [
        "A".repeat(64),
        "é".repeat(32),
        format!(" {}", "a".repeat(64)),
        "g".repeat(64),
    ] {
        options.usb_peer = Some(peer);
        assert!(options.target().is_err());
    }
}

#[test]
fn t977_profile_projection_retains_direction_processing_buffer_and_background() {
    for direction in [Direction::Microphone, Direction::Speakers] {
        for processing in [Processing::Speech, Processing::Raw] {
            for background in [false, true] {
                projection_boundaries(direction, processing, background);
            }
        }
    }
}
fn projection_boundaries(direction: Direction, processing: Processing, background: bool) {
    let resource = match direction {
        Direction::Microphone => Resource::Microphone,
        Direction::Speakers => Resource::Speakers,
    };
    for buffer_ms in (0..=220).chain([u16::MAX]) {
        let profile = AudioProfile {
            direction,
            processing,
            buffer_ms,
            background,
        };
        let mut options = AudioOptions::new(direction);
        options.profile = profile;
        assert_eq!(options.validate().is_ok(), profile.validate().is_ok());
        let projected = profile.invitation_profile();
        assert_eq!(
            projected.is_ok(),
            (20..=200).contains(&buffer_ms) && buffer_ms % 10 == 0
        );
        if let Ok((actual, projected)) = projected {
            assert_eq!(actual, resource);
            assert_eq!(
                projected,
                Audio {
                    processing: processing as u8,
                    buffer_ms,
                    background
                }
            );
            assert!(Profile::Audio(projected).validate(resource).is_ok());
            assert!(Profile::Audio(projected)
                .validate(Resource::Camera)
                .is_err());
        }
    }
}

#[test]
fn t977_saved_legacy_defaults_and_usb_roundtrips_are_passive() {
    let defaults: AudioSettings = toml::from_str("").unwrap();
    assert_eq!(defaults, AudioSettings::default());
    let legacy = r#"{"profile":{"direction":"microphone","processing":"raw","buffer_ms":20,"background":true},"serial":"original"}"#;
    let mut options: AudioOptions = serde_json::from_str(legacy).unwrap();
    assert_eq!(options.usb_peer, None);
    assert_eq!(options.target().unwrap(), Target::Adb(Some("original")));
    assert!(!serde_json::to_string(&options)
        .unwrap()
        .contains("usb_peer"));
    options.serial = None;
    options.usb_peer = Some("f".repeat(64));
    assert_eq!(
        serde_json::from_str::<AudioOptions>(&serde_json::to_string(&options).unwrap()).unwrap(),
        options
    );
    let mut settings = defaults.clone();
    settings.microphone = options;
    settings.speakers.usb_peer = Some("b".repeat(64));
    assert_eq!(
        toml::from_str::<AudioSettings>(&toml::to_string(&settings).unwrap()).unwrap(),
        settings
    );
    assert_eq!(
        defaults.microphone,
        AudioOptions::new(Direction::Microphone)
    );
    assert_eq!(defaults.speakers, AudioOptions::new(Direction::Speakers));
}

#[test]
fn t977_invalid_json_types_and_unknown_schema_are_rejected() {
    let mut value = serde_json::to_value(AudioOptions::new(Direction::Speakers)).unwrap();
    for field in ["direction", "processing"] {
        let original = value["profile"][field].clone();
        for invalid in [
            serde_json::json!(0),
            serde_json::json!(null),
            serde_json::json!("unknown"),
            serde_json::json!(true),
        ] {
            value["profile"][field] = invalid;
            assert!(serde_json::from_value::<AudioOptions>(value.clone()).is_err());
        }
        value["profile"][field] = original;
    }
    for invalid in [
        serde_json::json!(0),
        serde_json::json!(true),
        serde_json::json!(["a"]),
    ] {
        value["usb_peer"] = invalid;
        assert!(serde_json::from_value::<AudioOptions>(value.clone()).is_err());
    }
    value["usb_peer"] = serde_json::Value::Null;
    value["unknown"] = serde_json::json!(false);
    assert!(serde_json::from_value::<AudioOptions>(value).is_err());
}

#[cfg(feature = "platform")]
#[test]
fn t977_cli_selects_exact_transport_and_preserves_profile() {
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    let peer = "b".repeat(64);
    let cli = Cli::try_parse_from([
        "blent",
        "audio",
        "--direction",
        "speakers",
        "--processing",
        "raw",
        "--buffer-ms",
        "200",
        "--background",
        "--usb-peer",
        &peer,
    ])
    .unwrap();
    let Some(Commands::Audio(args)) = cli.command else {
        panic!("audio command")
    };
    let options = args.options();
    assert_eq!(
        options.profile,
        AudioProfile {
            direction: Direction::Speakers,
            processing: Processing::Raw,
            buffer_ms: 200,
            background: true
        }
    );
    assert!(matches!(options.target().unwrap(), Target::Usb(_)));
    assert!(
        Cli::try_parse_from(["blent", "audio", "--serial", "adb", "--usb-peer", &peer]).is_err()
    );
    let cli = Cli::try_parse_from(["blent", "audio", "--serial", "original"]).unwrap();
    let Some(Commands::Audio(args)) = cli.command else {
        panic!("audio command")
    };
    assert_eq!(
        args.options().target().unwrap(),
        Target::Adb(Some("original"))
    );
    for value in ["-1", "65536", "NaN", ""] {
        assert!(Cli::try_parse_from(["blent", "audio", "--buffer-ms", value]).is_err());
    }
}
