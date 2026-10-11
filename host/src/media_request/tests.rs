use super::*;
use blent_config::{
    audio::{AudioOptions, Direction},
    camera::CameraProfile,
    connection::pairing::PeerId,
};

#[test]
fn t991_camera_and_audio_requests_preserve_resource_profile_and_original_credential() {
    let token = "ab".repeat(32);
    for resource in [Resource::Camera, Resource::Microphone, Resource::Speakers] {
        let wire = if resource == Resource::Camera {
            Profile::Camera(CameraProfile::default().invitation_profile().unwrap())
        } else {
            Profile::Audio(blent_config::connection::management::Audio {
                processing: 2,
                buffer_ms: 200,
                background: true,
            })
        };
        let target = Target::Usb(PeerId::parse("cd".repeat(32)).unwrap());
        let projected = request(target, resource, wire, &token).unwrap();
        assert_eq!(projected.peer.as_str(), "cd".repeat(32));
        assert_eq!(projected.resource, resource);
        assert_eq!(projected.profile, wire);
        assert_eq!(projected.credential.as_bytes(), &[0xab; 32]);
    }
    for direction in [Direction::Microphone, Direction::Speakers] {
        let mut options = AudioOptions::new(direction);
        options.usb_peer = Some("cd".repeat(32));
        let (resource, profile) = options.profile.invitation_profile().unwrap();
        let projected = request(
            options.target().unwrap(),
            resource,
            Profile::Audio(profile),
            &token,
        )
        .unwrap();
        assert_eq!(projected.resource as u8, direction as u8 + 2);
    }
}
#[test]
fn t991_camera_and_audio_reject_native_selection_resource_and_profile_mismatch_before_registration()
{
    let audio = Profile::Audio(blent_config::connection::management::Audio {
        processing: 1,
        buffer_ms: 40,
        background: false,
    });
    let target = || Target::Usb(PeerId::parse("cd".repeat(32)).unwrap());
    let token = "ab".repeat(32);
    assert!(request(Target::Adb(None), Resource::Speakers, audio, &token).is_err());
    assert!(request(
        Target::Adb(Some("tablet")),
        Resource::Microphone,
        audio,
        &token
    )
    .is_err());
    assert!(request(target(), Resource::DisplaySession, Profile::Display, &token).is_err());
    assert!(request(target(), Resource::Camera, audio, &token).is_err());
    for resource in [Resource::Microphone, Resource::Speakers] {
        assert!(request(
            target(),
            resource,
            Profile::Camera(CameraProfile::default().invitation_profile().unwrap()),
            &token
        )
        .is_err());
        for processing in [0, 3, 255] {
            let invalid = Profile::Audio(blent_config::connection::management::Audio {
                processing,
                buffer_ms: 40,
                background: false,
            });
            assert!(request(target(), resource, invalid, &token).is_err());
        }
        for buffer_ms in [0, 19, 21, 201, u16::MAX] {
            let invalid = Profile::Audio(blent_config::connection::management::Audio {
                processing: 1,
                buffer_ms,
                background: false,
            });
            assert!(request(target(), resource, invalid, &token).is_err());
        }
    }
}
#[test]
fn t991_camera_and_audio_credential_byte_and_utf8_boundaries_are_checked_before_slicing() {
    for length in 0..=128 {
        let result = credential(&"a".repeat(length));
        assert_eq!(length == 64, result.is_ok());
    }
    for value in 0..=255u8 {
        let mut token = vec![b'a'; 64];
        token[31] = value;
        let token = String::from_utf8_lossy(&token);
        assert_eq!(valid_credential(&token), credential(&token).is_ok());
    }
    for bad in [
        "é".repeat(32),
        "😀".repeat(16),
        "A".repeat(64),
        "z".repeat(64),
        "\0".repeat(64),
    ] {
        assert!(!valid_credential(&bad));
        assert!(credential(&bad).is_err());
    }
    assert_eq!(credential(&"01".repeat(32)).unwrap().as_bytes(), &[1; 32]);
}

#[test]
fn t991_camera_and_audio_original_requests_reject_mismatch_and_invalid_schema() {
    let target = || Target::Usb(PeerId::parse("cd".repeat(32)).unwrap());
    let wire = Profile::Audio(blent_config::connection::management::Audio {
        processing: 1,
        buffer_ms: 40,
        background: false,
    });
    let selected = request(target(), Resource::Speakers, wire, &"ab".repeat(32)).unwrap();
    validate_original(&selected, &"ab".repeat(32)).unwrap();
    for index in 0..32 {
        let mut forged = selected.clone();
        let mut bytes = [0xab; 32];
        bytes[index] ^= 1;
        forged.credential = Credential::new(bytes);
        assert!(validate_original(&forged, &"ab".repeat(32)).is_err());
    }
    for token in ["", "😀", "ZZ", &"a".repeat(65)] {
        assert!(validate_original(&selected, token).is_err());
    }
    let mut invalid = selected.clone();
    invalid.resource = Resource::DisplaySession;
    invalid.profile = Profile::Display;
    assert!(validate_original(&invalid, &"ab".repeat(32)).is_err());
    invalid.resource = Resource::Speakers;
    assert!(validate_original(&invalid, &"ab".repeat(32)).is_err());
}
