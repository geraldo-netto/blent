//! T1553: persistence validates each media target before publication.
use super::*;

#[test]
fn t1650_saved_audio_direction_must_match_its_resource() {
    use crate::audio::Direction;
    for microphone in [false, true] {
        let mut config = FileConfig::default();
        if microphone {
            config.audio.microphone.profile.direction = Direction::Speakers;
        } else {
            config.audio.speakers.profile.direction = Direction::Microphone;
        }
        let restored: FileConfig = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert!(
            restored.validate().is_err(),
            "T1650: inconsistent audio resource admitted"
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        assert!(crate::storage::save_at(&restored, &path).is_err());
        assert!(!path.exists());
    }
}

fn target(resource: u8, serial: Option<String>, peer: Option<String>) -> FileConfig {
    let mut config = FileConfig::default();
    let (saved_serial, saved_peer) = match resource {
        0 => (
            &mut config.camera.options.serial,
            &mut config.camera.options.usb_peer,
        ),
        1 => (
            &mut config.audio.microphone.serial,
            &mut config.audio.microphone.usb_peer,
        ),
        2 => (
            &mut config.audio.speakers.serial,
            &mut config.audio.speakers.usb_peer,
        ),
        _ => panic!("invalid fixture resource"),
    };
    *saved_serial = serial;
    *saved_peer = peer;
    config
}

#[test]
fn t1553_all_saved_media_fingerprints_require_exact_valid_identity() {
    for resource in 0..3 {
        for length in 0..=65 {
            let config = target(resource, None, Some("a".repeat(length)));
            assert_eq!(
                config.validate().is_ok(),
                length == 64,
                "T1553 resource={resource} length={length}"
            );
        }
        assert!(target(resource, None, Some("g".repeat(64)))
            .validate()
            .is_err());
        assert!(target(resource, Some("USB".into()), Some("a".repeat(64)))
            .validate()
            .is_err());
    }
}

#[test]
fn t1553_all_saved_media_serials_reject_invalid_boundaries() {
    for resource in 0..3 {
        assert!(target(resource, None, None).validate().is_ok());
        assert!(target(resource, Some("USB".into()), None)
            .validate()
            .is_ok());
        assert!(target(resource, Some("x".repeat(1024)), None)
            .validate()
            .is_ok());
        for serial in ["".to_owned(), "USB\0bad".into(), "x".repeat(1025)] {
            assert!(
                target(resource, Some(serial), None).validate().is_err(),
                "T1553 resource={resource}"
            );
        }
    }
}

#[test]
fn t1553_invalid_media_save_preserves_the_previous_configuration() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let store = crate::storage::ConfigStore::new(path.clone());
    store.save(&FileConfig::default()).unwrap();
    let original = std::fs::read(&path).unwrap();
    for resource in 0..3 {
        let invalid = target(resource, None, Some(String::new()));
        assert!(store.save(&invalid).is_err(), "T1553 resource={resource}");
        assert!(store
            .update(|config| {
                *config = invalid;
                Ok(())
            })
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}
