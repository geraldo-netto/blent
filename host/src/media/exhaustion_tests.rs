//! T1187: terminal decoder scopes cannot revive previous selection authority.
use super::tests::selection_fixture;
use super::*;

fn selected(key: crate::selection::Key) -> crate::selection::Selected {
    crate::selection::Selected {
        key,
        workers: 2,
        encoder: "h264_vaapi".into(),
        reason: "original measured result".into(),
        verified: true,
        calibrating: false,
        decoder: None,
    }
}

#[test]
fn t1187_terminal_clear_pins_counter_and_discards_original_metadata() {
    let mut settings = selection_fixture();
    settings.decoder_epoch = u64::MAX;
    settings.decoders = Some(DecoderCapabilities::default());
    settings.selection = Some(selected(settings.selection_key()));
    for _ in 0..3 {
        settings.clear_decoders();
        assert_eq!(
            settings.decoder_epoch,
            u64::MAX,
            "T1187 terminal clear wrapped"
        );
        assert!(settings.decoders.is_none());
        assert!(settings.selection.is_none());
    }
}

#[test]
fn t1187_old_zero_selection_cannot_revive_after_terminal_clear() {
    let mut settings = selection_fixture();
    settings.decoder_epoch = 0;
    let original = selected(settings.selection_key());
    settings.decoder_epoch = u64::MAX;
    settings.clear_decoders();
    settings.selection = Some(original.clone());
    assert!(
        !settings.selection_matches(&original.key),
        "T1187 zero key revived"
    );
    assert_eq!(settings.selected_workers(), 0);
    assert_eq!(settings.effective_encoder(), "libx264");
}

#[test]
fn t1187_terminal_snapshot_cannot_certify_selection_or_request_calibration() {
    let mut settings = selection_fixture();
    settings.decoder_epoch = u64::MAX;
    let terminal = settings.selection_key();
    settings.selection = Some(selected(terminal.clone()));
    assert!(
        !settings.selection_matches(&terminal),
        "T1187 sentinel granted selection"
    );
    assert_eq!(settings.selected_workers(), 0);
    assert_eq!(settings.effective_encoder(), "libx264");
    assert!(!settings.calibrating());
    assert!(settings.selection_reason().contains("exhausted"));
}

#[test]
fn t1187_bounded_resets_follow_wider_integer_terminal_model() {
    for initial in (0..=64).chain(u64::MAX - 64..=u64::MAX) {
        let mut settings = selection_fixture();
        settings.decoder_epoch = initial;
        let mut expected = u128::from(initial);
        for _ in 0..80 {
            expected = (expected + 1).min(u128::from(u64::MAX));
            settings.clear_decoders();
            assert_eq!(
                u128::from(settings.decoder_epoch),
                expected,
                "T1187 initial={initial}"
            );
            let key = settings.selection_key();
            assert_eq!(
                settings.selection_matches(&key),
                expected < u128::from(u64::MAX)
            );
        }
    }
}

#[test]
fn t1187_terminal_retained_capabilities_cannot_admit_any_codec() {
    let mut settings = selection_fixture();
    settings.decoder_epoch = u64::MAX;
    let mut caps: DecoderCapabilities = serde_json::from_str(include_str!(
        "../../../testdata/decoder-capabilities-v2.json"
    ))
    .unwrap();
    (caps.width, caps.height) = settings.video_dimensions();
    caps.scope = Some(u64::MAX.to_string());
    assert!(caps.valid());
    settings.decoders = Some(caps);
    for codec in Codec::ALL {
        assert!(
            !settings.decoder_supports(codec),
            "T1187 terminal codec admitted: {codec:?}"
        );
    }
}
