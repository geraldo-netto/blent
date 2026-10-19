use super::*;
use blent_config::negotiation::{DecoderChoice, Profile, StreamProfile};

fn choice(codec: &str, depth: u8) -> DecoderChoice {
    DecoderChoice {
        name: "original-decoder".into(),
        stream: StreamProfile {
            codec: codec.into(),
            format: Profile {
                profile: "fixture-profile".into(),
                level: 31,
                depth,
            },
        },
        low_latency: false,
        operating_rate: None,
    }
}

fn candidate(
    name: &str,
    format: (u32, u32, u32, u32, u32),
    decoder: Option<DecoderChoice>,
) -> (CaptureConfig, std::sync::Arc<EncoderEvidence>) {
    let config = CaptureConfig {
        encoder: name.into(),
        ..Default::default()
    };
    let tracker = crate::latency::LatencyTracker::new();
    let evidence = tracker.encoder_started_with_decoder(name, format, decoder);
    (config, evidence)
}

#[test]
fn t1115_cpu_cli_recipe_gate_preserves_explicit_backend_and_decoder_limits() {
    for (name, codec) in [
        ("libx264", "h264"),
        ("libvpx-vp9", "vp9"),
        ("libaom-av1", "av1"),
    ] {
        let (mut config, evidence) =
            candidate(name, (64, 64, 60, 20000, 18), Some(choice(codec, 8)));
        assert!(qualified(&config, &evidence));
        config.ten_bit = true;
        assert!(!qualified(&config, &evidence));
    }
    for name in [
        "h264_nvenc",
        "h264_vaapi_baseline",
        "hevc_nvenc",
        "unknown",
        "",
    ] {
        let (config, evidence) = candidate(name, (64, 64, 60, 20000, 18), Some(choice("h264", 8)));
        assert!(!qualified(&config, &evidence));
    }
    for decoder in [
        None,
        Some(choice("vp9", 8)),
        Some(choice("h264", 10)),
        Some(choice("h264", 0)),
    ] {
        let (config, evidence) = candidate("libx264", (64, 64, 60, 20000, 18), decoder);
        assert!(!qualified(&config, &evidence));
    }
    let mut unnamed = choice("h264", 8);
    unnamed.name.clear();
    let (config, evidence) = candidate("libx264", (64, 64, 60, 20000, 18), Some(unnamed));
    assert!(!qualified(&config, &evidence));
}

#[test]
fn t1115_recipe_trial_geometry_bounds_have_independent_expected_results() {
    for (edge, allowed) in [
        (0, false),
        (1, false),
        (2, true),
        (3, false),
        (4096, true),
        (4097, false),
        (u32::MAX, false),
    ] {
        for (width, height) in [(edge, 64), (64, edge)] {
            let (config, evidence) = candidate(
                "libx264",
                (width, height, 60, 20000, 18),
                Some(choice("h264", 8)),
            );
            assert_eq!(
                qualified(&config, &evidence),
                allowed,
                "T1115 geometry {width}x{height}"
            );
        }
    }
}

#[test]
fn t1115_recipe_numeric_inputs_reject_out_of_contract_values() {
    let invalid = [
        (0, 20000, 18),
        (blent_config::MAX_FPS + 1, 20000, 18),
        (u32::MAX, 20000, 18),
        (60, 0, 18),
        (60, blent_config::MAX_BITRATE_KBPS + 1, 18),
        (60, u32::MAX, 18),
        (60, 20000, blent_config::MAX_QUALITY + 1),
        (60, 20000, u32::MAX),
    ];
    for (fps, bitrate, quality) in invalid {
        let (config, evidence) = candidate(
            "libx264",
            (64, 64, fps, bitrate, quality),
            Some(choice("h264", 8)),
        );
        assert!(!qualified(&config, &evidence));
    }
    let valid = [
        (1, 1, blent_config::MIN_QUALITY),
        (
            blent_config::MAX_FPS,
            blent_config::MAX_BITRATE_KBPS,
            blent_config::MAX_QUALITY,
        ),
    ];
    for (fps, bitrate, quality) in valid {
        let (config, evidence) = candidate(
            "libx264",
            (64, 64, fps, bitrate, quality),
            Some(choice("h264", 8)),
        );
        assert!(qualified(&config, &evidence));
    }
}

#[test]
fn t1115_policy_choice_has_no_implicit_qualification_or_liveness_grant() {
    let conditions = unchanged::Conditions::default();
    for mode in [Mode::Compatible, Mode::Adaptive, Mode::Unchanged] {
        let mut policy = Policy::new(mode, false, false);
        assert_eq!(policy.phase(), Phase::Rejected);
        assert_eq!(policy.interval_ms(), 200);
        policy.reject();
        policy.tick(conditions, 0);
        policy.observe(
            conditions,
            Sample {
                sequence: 0,
                pts_us: 0,
                ready_us: 0,
                ack_us: 0,
                keyframe: true,
            },
        );
        assert_eq!(policy.phase(), Phase::Rejected);
    }
    assert_eq!(Mode::default(), Mode::Compatible);
}
