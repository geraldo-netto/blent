//! T1678: independent forward rotation, then mirror and nearest-neighbor placement.
//! Production reads inverse coordinates; this oracle materializes the turned plane.
use super::*;

fn destination(x: usize, y: usize, size: (usize, usize), rotation: u16) -> (usize, usize) {
    match rotation {
        90 => (size.1 - 1 - y, x),
        180 => (size.0 - 1 - x, size.1 - 1 - y),
        270 => (y, size.0 - 1 - x),
        _ => (x, y),
    }
}

fn turned(source: &[u8], size: (usize, usize), image: Image) -> (Vec<u8>, usize) {
    let width = if image.rotation.is_multiple_of(180) {
        size.0
    } else {
        size.1
    };
    let mut result = vec![0; source.len()];
    for y in 0..size.1 {
        for x in 0..size.0 {
            let (column, row) = destination(x, y, size, image.rotation);
            result[row * width + column] = source[y * size.0 + x];
        }
    }
    if image.mirror {
        for row in result.chunks_exact_mut(width) {
            row.reverse();
        }
    }
    (result, width)
}

fn expected(source: &[u8], size: (usize, usize), rect: Rect, image: Image, black: u8) -> Vec<u8> {
    let (rotated, width) = turned(source, size, image);
    let height = rotated.len() / width;
    let mut result = vec![black; source.len()];
    for row in 0..rect.height {
        let source_row = row * height / rect.height;
        for column in 0..rect.width {
            result[(row + rect.top) * size.0 + column + rect.left] =
                rotated[source_row * width + column * width / rect.width];
        }
    }
    result
}

fn verify(profile: CameraProfile, quarter_rect: Rect) {
    let size = (profile.width as usize, profile.height as usize);
    let source: Vec<_> = (0..profile.frame_bytes().unwrap())
        .map(|index| (index % 251) as u8)
        .collect();
    let mut transform = Transform::new(&profile).unwrap();
    let pixels = size.0 * size.1;
    for rotation in [270, 0, 180, 90, 0] {
        let rect = if rotation % 180 == 0 {
            Rect {
                left: 0,
                top: 0,
                width: size.0,
                height: size.1,
            }
        } else {
            quarter_rect
        };
        for mirror in [true, false] {
            let image = Image {
                rotation,
                mirror,
                ..Image::from_profile(&profile)
            };
            let mut output = source.clone();
            transform.apply(image, &mut output).unwrap();
            assert_eq!(
                output[..pixels],
                expected(&source[..pixels], size, rect, image, 16)
            );
            verify_chroma(&source[pixels..], &output[pixels..], size, rect, image);
        }
    }
}

fn verify_chroma(source: &[u8], output: &[u8], size: (usize, usize), rect: Rect, image: Image) {
    let chroma_size = (size.0 / 2, size.1 / 2);
    let chroma_rect = Rect {
        left: rect.left / 2,
        top: rect.top / 2,
        width: rect.width / 2,
        height: rect.height / 2,
    };
    for (input, actual) in source
        .chunks_exact(chroma_size.0 * chroma_size.1)
        .zip(output.chunks_exact(chroma_size.0 * chroma_size.1))
    {
        assert_eq!(
            actual,
            expected(input, chroma_size, chroma_rect, image, 128)
        );
    }
}

#[test]
fn t1678_every_plane_matches_forward_rotation_oracle_across_live_changes() {
    // Expected letterbox rectangles are fixed independently of placement().
    for (width, height, left, top, w, h) in [
        (160, 120, 34, 0, 90, 120),
        (162, 122, 36, 0, 90, 122),
        (160, 1080, 0, 528, 160, 22),
        (1920, 120, 956, 0, 6, 120),
        (160, 160, 0, 0, 160, 160),
    ] {
        verify(
            CameraProfile {
                width,
                height,
                ..Default::default()
            },
            Rect {
                left,
                top,
                width: w,
                height: h,
            },
        );
    }
}

#[test]
fn t1678_invalid_frame_bounds_reject_without_modifying_pixels() {
    let profile = CameraProfile::default();
    let mut transform = Transform::new(&profile).unwrap();
    let bytes = profile.frame_bytes().unwrap();
    for size in [0, 1, bytes - 1, bytes + 1] {
        let mut frame = vec![71; size];
        assert!(transform
            .apply(Image::from_profile(&profile), &mut frame)
            .is_err());
        assert!(frame.iter().all(|value| *value == 71));
    }
}
