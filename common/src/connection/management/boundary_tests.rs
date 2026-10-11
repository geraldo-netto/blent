//! T981: bounded shared wire/schema inputs, including every unknown byte tag.
use super::*;

#[test]
fn t981_every_resource_byte_accepts_only_the_fixed_wire_mapping() {
    let known = [
        (1, Resource::DisplaySession),
        (2, Resource::Camera),
        (3, Resource::Microphone),
        (4, Resource::Speakers),
    ];
    for raw in 0..=u8::MAX {
        let expected = known
            .iter()
            .find(|(tag, _)| *tag == raw)
            .map(|(_, value)| *value);
        assert_eq!(
            Resource::try_from(raw).ok(),
            expected,
            "T981 resource tag {raw}"
        );
        if let Some(value) = expected {
            assert_eq!(value as u8, raw);
        }
    }
}

#[test]
fn t981_every_outcome_byte_accepts_only_the_fixed_wire_mapping() {
    let known = [
        (0, Outcome::Admitted),
        (1, Outcome::Unavailable),
        (2, Outcome::PermissionRequired),
        (3, Outcome::PermissionDenied),
        (4, Outcome::Busy),
        (5, Outcome::Revoked),
        (6, Outcome::Failed),
    ];
    for raw in 0..=u8::MAX {
        let expected = known
            .iter()
            .find(|(tag, _)| *tag == raw)
            .map(|(_, value)| *value);
        assert_eq!(
            Outcome::try_from(raw).ok(),
            expected,
            "T981 outcome tag {raw}"
        );
        if let Some(value) = expected {
            assert_eq!(value as u8, raw);
        }
    }
}

#[test]
fn t981_every_reason_byte_accepts_only_the_fixed_wire_mapping() {
    let known = [
        (0, Reason::Requested),
        (1, Reason::ConnectionLost),
        (2, Reason::WorkerFailed),
        (3, Reason::TimedOut),
        (4, Reason::PermissionRevoked),
    ];
    for raw in 0..=u8::MAX {
        let expected = known
            .iter()
            .find(|(tag, _)| *tag == raw)
            .map(|(_, value)| *value);
        assert_eq!(
            Reason::try_from(raw).ok(),
            expected,
            "T981 reason tag {raw}"
        );
        if let Some(value) = expected {
            assert_eq!(value as u8, raw);
        }
    }
}

#[test]
fn t981_audio_profile_boundaries_reject_invalid_values_and_other_resource_kinds() {
    for processing in 0..=u8::MAX {
        for buffer_ms in [0, 1, 19, 20, 21, 40, 199, 200, 201, u16::MAX] {
            for background in [false, true] {
                let audio = Audio {
                    processing,
                    buffer_ms,
                    background,
                };
                let valid = matches!(processing, 1 | 2) && matches!(buffer_ms, 20 | 40 | 200);
                assert_eq!(audio.validate().is_ok(), valid);
                assert_eq!(
                    Profile::Audio(audio).validate(Resource::Microphone).is_ok(),
                    valid
                );
                assert_eq!(
                    Profile::Audio(audio).validate(Resource::Speakers).is_ok(),
                    valid
                );
                assert!(Profile::Audio(audio).validate(Resource::Camera).is_err());
                assert!(Profile::Audio(audio)
                    .validate(Resource::DisplaySession)
                    .is_err());
            }
        }
    }
    for buffer_ms in 20..=200 {
        let audio = Audio {
            processing: 1,
            buffer_ms,
            background: false,
        };
        assert_eq!(audio.validate().is_ok(), buffer_ms % 10 == 0);
    }
}
