use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[tokio::test]
async fn t993_audio_short_and_utf8_tokens_return_errors_before_any_native_spawn() {
    let profile = AudioProfile::new(Direction::Speakers);
    let tokens = [
        String::new(),
        "short".into(),
        "a".repeat(11),
        format!("a{}", "é".repeat(32)),
        format!("abc{}", "😀".repeat(16)),
    ];
    for token in tokens {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            spawn_with_clock(Path::new("/bin/false"), profile, &token, true)
        }));
        assert!(
            outcome.is_ok(),
            "T993 audio token panicked at UTF-8/range boundary"
        );
        assert!(
            outcome.unwrap().is_err(),
            "T993 malformed token reached native spawn"
        );
    }
}
#[tokio::test]
async fn t993_audio_invalid_profiles_reject_before_any_native_spawn() {
    for direction in [Direction::Microphone, Direction::Speakers] {
        for buffer_ms in [0, 19, 21, 201, u16::MAX] {
            let mut profile = AudioProfile::new(direction);
            profile.buffer_ms = buffer_ms;
            assert!(
                spawn_with_clock(Path::new("/bin/false"), profile, &"ab".repeat(32), false)
                    .is_err(),
                "T993 invalid audio profile reached native spawn"
            );
        }
    }
}

#[tokio::test]
async fn t993_audio_bounded_invalid_credential_bytes_and_lengths_never_reach_helper() {
    let profile = AudioProfile::new(Direction::Microphone);
    for length in 0..=128 {
        if length == 64 {
            continue;
        }
        assert!(
            spawn_with_clock(Path::new("/bin/false"), profile, &"a".repeat(length), false).is_err()
        );
    }
    for byte in 0..=255u8 {
        if byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) {
            continue;
        }
        let mut bytes = vec![b'a'; 64];
        bytes[11] = byte;
        let token = String::from_utf8_lossy(&bytes);
        assert!(spawn_with_clock(Path::new("/bin/false"), profile, &token, true).is_err());
    }
}
