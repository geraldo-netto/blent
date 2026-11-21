use super::*;
fn profile() -> CameraProfile {
    CameraProfile {
        width: 160,
        height: 120,
        ..Default::default()
    }
}
#[test]
fn t1467_identity_mirror_and_half_turn_have_independent_pixel_oracles() {
    let profile = profile();
    let mut transform = Transform::new(&profile).unwrap();
    let source: Vec<_> = (0..profile.frame_bytes().unwrap())
        .map(|i| (i % 251) as u8)
        .collect();
    let mut image = Image::from_profile(&profile);
    for (rotation, mirror) in [(0, false), (0, true), (180, false), (180, true)] {
        image.rotation = rotation;
        image.mirror = mirror;
        let mut output = source.clone();
        transform.apply(image, &mut output).unwrap();
        for y in 0..120 {
            for x in 0..160 {
                let ox = if mirror ^ (rotation == 180) {
                    159 - x
                } else {
                    x
                };
                let oy = if rotation == 180 { 119 - y } else { y };
                assert_eq!(output[y * 160 + x], source[oy * 160 + ox]);
            }
        }
    }
}
#[test]
fn t1467_quarter_turns_keep_layout_and_chroma_aligned_letterboxing() {
    for (width, height) in [(160, 120), (162, 122), (160, 1080), (1920, 120)] {
        let profile = CameraProfile {
            width,
            height,
            ..Default::default()
        };
        let mut transform = Transform::new(&profile).unwrap();
        let mut image = Image::from_profile(&profile);
        for rotation in [90, 270] {
            image.rotation = rotation;
            let mut frame = vec![90; profile.frame_bytes().unwrap()];
            transform.apply(image, &mut frame).unwrap();
            let pixels = (width * height) as usize;
            assert!(frame[..pixels].iter().all(|v| [16, 90].contains(v)));
            assert!(frame[pixels..].iter().all(|v| [128, 90].contains(v)));
            assert!(frame[..pixels].contains(&90));
        }
    }
}
#[test]
fn t1467_invalid_transform_is_atomic() {
    let profile = profile();
    let mut transform = Transform::new(&profile).unwrap();
    let mut image = Image::from_profile(&profile);
    let mut frame = vec![70; profile.frame_bytes().unwrap()];
    for rotation in [1, 89, 360, u16::MAX] {
        image.rotation = rotation;
        assert!(transform.apply(image, &mut frame).is_err());
        assert!(frame.iter().all(|v| *v == 70));
    }
    image.rotation = 0;
    assert!(transform.apply(image, &mut frame[..100]).is_err());
    assert!(Transform::new(&CameraProfile {
        width: u32::MAX,
        ..profile
    })
    .is_err());
}

#[test]
fn t1467_quarter_turn_quadrants_match_independent_orientation_oracle() {
    let profile = profile();
    let mut transform = Transform::new(&profile).unwrap();
    let mut source = vec![128; profile.frame_bytes().unwrap()];
    for y in 0..120 {
        for x in 0..160 {
            source[y * 160 + x] =
                [30, 60, 90, 120][usize::from(x >= 80) + 2 * usize::from(y >= 60)];
        }
    }
    for (rotation, expected) in [(90, [90, 30, 120, 60]), (270, [60, 120, 30, 90])] {
        let mut image = Image::from_profile(&profile);
        image.rotation = rotation;
        let mut output = source.clone();
        transform.apply(image, &mut output).unwrap();
        assert_eq!(
            [
                output[34],
                output[123],
                output[119 * 160 + 34],
                output[119 * 160 + 123]
            ],
            expected
        );
        assert_eq!(output[0], 16);
    }
}
