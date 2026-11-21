use super::*;

fn profile() -> CameraProfile {
    CameraProfile {
        width: 160,
        height: 120,
        shadow_mode: crate::camera::ShadowMode::Auto,
        ..Default::default()
    }
}

fn frame(profile: &CameraProfile, luma: u8) -> Vec<u8> {
    let mut frame = vec![128; profile.frame_bytes().unwrap()];
    frame[..(profile.width * profile.height) as usize].fill(luma);
    frame
}

#[test]
fn t1450_dark_center_bright_background_and_chroma() {
    let profile = profile();
    let mut adjustment = AutoBrightness::new(&profile).unwrap();
    let mut source = frame(&profile, 235);
    for y in 30..90 {
        source[y * 160 + 40..y * 160 + 120].fill(40);
    }
    for _ in 0..60 {
        let mut output = source.clone();
        adjustment.apply(&mut output).unwrap();
        assert!(output[60 * 160 + 80] > 40);
        assert_eq!(output[0], 235);
        assert_eq!(&output[19200..], &source[19200..]);
    }
    assert!((0.45..1.0).contains(&adjustment.gamma));
}

#[test]
fn t1450_bright_and_black_frames_stay_unchanged() {
    for luma in [0, 16, 128, 180, 235, 255] {
        let profile = profile();
        let mut adjustment = AutoBrightness::new(&profile).unwrap();
        let original = frame(&profile, luma);
        let mut output = original.clone();
        adjustment.apply(&mut output).unwrap();
        assert_eq!(output, original);
    }
}

#[test]
fn t1450_curve_is_bounded_monotonic_and_never_darkens() {
    for step in 0..=100 {
        let gamma = 0.45 + 0.55 * step as f32 / 100.0;
        let table = curve(gamma, 100);
        assert_eq!(&table[..17], &(0..=16).collect::<Vec<u8>>());
        assert_eq!(&table[235..], &(235..=255).collect::<Vec<u8>>());
        assert!(table.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(table
            .iter()
            .enumerate()
            .all(|(input, output)| *output as usize >= input));
    }
}

#[test]
fn t1450_invalid_dimensions_fps_and_frames_are_atomic() {
    for value in [0, 1, 119, 121, 159, 161, 1921, u32::MAX] {
        assert!(AutoBrightness::new(&CameraProfile {
            width: value,
            ..profile()
        })
        .is_err());
    }
    for fps in [0, 4, 31, u32::MAX] {
        assert!(AutoBrightness::new(&CameraProfile { fps, ..profile() }).is_err());
    }
    let mut adjustment = AutoBrightness::new(&profile()).unwrap();
    for size in [0, 1, 28799, 28801, 40000] {
        let mut bytes = vec![31; size];
        assert!(adjustment.apply(&mut bytes).is_err());
        assert!(bytes.iter().all(|value| *value == 31));
        assert_eq!(adjustment.gamma, 1.0);
    }
}

#[test]
fn t1450_temporal_smoothing_and_recovery_are_bounded() {
    for fps in [5, 15, 30] {
        let profile = CameraProfile { fps, ..profile() };
        let mut adjustment = AutoBrightness::new(&profile).unwrap();
        for level in [32, 200, 40, 235] {
            for _ in 0..fps * 2 {
                let previous = adjustment.gamma;
                adjustment.apply(&mut frame(&profile, level)).unwrap();
                assert!((0.45..=1.0).contains(&adjustment.gamma));
                assert!((adjustment.gamma - previous).abs() <= 0.55 * adjustment.blend + 0.00001);
            }
        }
        assert!(adjustment.gamma > 0.99);
    }
}

#[cfg(feature = "platform")]
#[test]
fn t1450_t1466_defaults_saved_and_cli_selectable() {
    use clap::Parser;
    assert_eq!(CameraProfile::default().shadow_mode, ShadowMode::Auto);
    assert_eq!(CameraProfile::default().shadow_strength, 40);
    let profile: CameraProfile =
        toml::from_str("shadow_mode = \"manual\"\nshadow_strength = 65").unwrap();
    assert_eq!(profile.shadow_mode, ShadowMode::Manual);
    let encoded = toml::to_string(&profile).unwrap();
    assert_eq!(toml::from_str::<CameraProfile>(&encoded).unwrap(), profile);
    let parsed = crate::cli::Cli::parse_from([
        "blent",
        "cameras",
        "--shadow-mode",
        "manual",
        "--shadow-strength",
        "65",
    ]);
    let Some(crate::cli::Commands::Cameras(options)) = parsed.command else {
        panic!()
    };
    assert_eq!(options.shadow_mode, ShadowMode::Manual);
    assert_eq!(options.shadow_strength, 65);
}

#[test]
fn t1466_shadow_lift_protects_upper_midtones() {
    let profile = profile();
    let mut adjustment = AutoBrightness::new(&profile).unwrap();
    let mut original = frame(&profile, 40);
    original[0] = 200;
    for _ in 0..60 {
        let mut output = original.clone();
        adjustment.apply(&mut output).unwrap();
        assert!(
            output[0] <= 203,
            "T1466 bright background lifted excessively: {}",
            output[0]
        );
        assert!(output[60 * 160 + 80] > 40);
    }
}

#[test]
fn t1466_modes_strength_and_invalid_updates_are_bounded() {
    let profile = profile();
    let mut adjustment = AutoBrightness::new(&profile).unwrap();
    for mode in [ShadowMode::Auto, ShadowMode::Manual, ShadowMode::Off] {
        for strength in [0, 40, 100] {
            adjustment.update(mode, strength).unwrap();
            let mut output = frame(&profile, 40);
            adjustment.apply(&mut output).unwrap();
            assert!(output[..19200].iter().all(|v| (40..=235).contains(v)));
            if mode == ShadowMode::Off || strength == 0 {
                assert_eq!(output[0], 40);
            }
        }
    }
    for invalid in [101, 128, 255] {
        assert!(adjustment.update(ShadowMode::Auto, invalid).is_err());
        assert_eq!(adjustment.mode, ShadowMode::Off);
        assert!(AutoBrightness::new(&CameraProfile {
            shadow_strength: invalid,
            ..profile.clone()
        })
        .is_err());
    }
}
