//! T1678: bounded CPU-only release benchmark, no camera/transport/native handles.
use blent_config::camera::{adjustments::Image, transform::Transform, CameraProfile};
use std::{hint::black_box, time::Instant};

fn sample(width: u32, height: u32, rotation: u16, mirror: bool, repeat: usize) {
    let profile = CameraProfile {
        width,
        height,
        ..Default::default()
    };
    let source: Vec<_> = (0..profile.frame_bytes().unwrap())
        .map(|i| (i % 251) as u8)
        .collect();
    let mut frame = source.clone();
    let mut transform = Transform::new(&profile).unwrap();
    let mut image = Image::from_profile(&profile);
    image.rotation = rotation;
    image.mirror = mirror;
    let mut elapsed = 0;
    for _ in 0..60 {
        frame.copy_from_slice(&source);
        let start = Instant::now();
        transform
            .apply(black_box(image), black_box(&mut frame))
            .unwrap();
        elapsed += start.elapsed().as_nanos();
        black_box(&frame);
    }
    let checksum = frame.iter().fold(14695981039346656037_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
    });
    println!(
        "{}",
        serde_json::json!({"width":width,"height":height,"rotation":rotation,"mirror":mirror,"repeat":repeat,"frames":60,"elapsed_ns":elapsed as u64,"checksum_fnv1a64":checksum})
    );
}

fn main() {
    for repeat in 0..5 {
        for (width, height) in [(1280, 720), (1920, 1080)] {
            for step in 0..4 {
                let rotation = ((step + repeat) % 4 * 90) as u16;
                for mirror in [repeat % 2 == 0, repeat % 2 != 0] {
                    sample(width, height, rotation, mirror, repeat);
                }
            }
        }
    }
}
