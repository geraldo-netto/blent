//! Bounded, portable RGBA thumbnail of the already-transformed webcam output.
use super::Lens;
use anyhow::{ensure, Result};

const SIDE: usize = 640;

#[derive(Debug)]
pub struct CameraPreview {
    lens: Lens,
    size: [usize; 2],
    rgba: Vec<u8>,
    created: std::time::Instant,
}

impl CameraPreview {
    pub fn new(lens: Lens, size: [usize; 2], rgba: Vec<u8>) -> Result<Self> {
        ensure!(
            size.iter().all(|side| (1..=SIDE).contains(side)),
            "camera preview dimensions exceed bounds"
        );
        ensure!(
            rgba.len() == size[0] * size[1] * 4,
            "camera preview buffer size mismatch"
        );
        Ok(Self {
            lens,
            size,
            rgba,
            created: std::time::Instant::now(),
        })
    }
    pub fn fresh(&self) -> bool {
        std::time::Instant::now()
            .checked_duration_since(self.created)
            .is_some_and(|age| age < super::FRESH_FRAME_AGE)
    }
    pub fn lens(&self) -> Lens {
        self.lens
    }
    pub fn size(&self) -> [usize; 2] {
        self.size
    }
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn from_yuv420(
        lens: Lens,
        width: u32,
        height: u32,
        bytes: &[u8],
        published: std::time::Instant,
    ) -> Result<Self> {
        validate_yuv(width, height, bytes.len())?;
        let (width, height) = (width as usize, height as usize);
        let longest = width.max(height);
        let target = longest.min(SIDE);
        let size = [
            (width * target / longest).max(1),
            (height * target / longest).max(1),
        ];
        let mut rgba = Vec::with_capacity(size[0] * size[1] * 4);
        for y in 0..size[1] {
            for x in 0..size[0] {
                rgba.extend(pixel(
                    bytes,
                    width,
                    height,
                    x * width / size[0],
                    y * height / size[1],
                ));
            }
        }
        let mut preview = Self::new(lens, size, rgba)?;
        preview.created = published;
        Ok(preview)
    }
}

fn validate_yuv(width: u32, height: u32, length: usize) -> Result<()> {
    ensure!(
        (2..=1920).contains(&width) && (2..=1080).contains(&height),
        "invalid camera preview source dimensions"
    );
    ensure!(
        width.is_multiple_of(2) && height.is_multiple_of(2),
        "camera preview requires even YUV dimensions"
    );
    ensure!(
        length == (width * height * 3 / 2) as usize,
        "camera preview source buffer size mismatch"
    );
    Ok(())
}

fn pixel(bytes: &[u8], width: usize, height: usize, x: usize, y: usize) -> [u8; 4] {
    let luma = width * height;
    let chroma = (y / 2) * (width / 2) + x / 2;
    let c = i32::from(bytes[y * width + x]) - 16;
    let d = i32::from(bytes[luma + chroma]) - 128;
    let e = i32::from(bytes[luma + luma / 4 + chroma]) - 128;
    let channel = |value: i32| ((value + 128) >> 8).clamp(0, 255) as u8;
    [
        channel(298 * c + 409 * e),
        channel(298 * c - 100 * d - 208 * e),
        channel(298 * c + 516 * d),
        255,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t543_preview_bounds_colours_and_shape_preserve_exported_pixels() {
        let mut frame = CameraPreview::from_yuv420(
            Lens::Rear,
            2,
            2,
            &[16, 235, 81, 145, 128, 128],
            std::time::Instant::now(),
        )
        .unwrap();
        assert!(frame.fresh());
        frame.created -= std::time::Duration::from_secs(2);
        assert!(!frame.fresh());
        assert_eq!(frame.size(), [2, 2]);
        assert_eq!(frame.lens(), Lens::Rear);
        assert_eq!(&frame.rgba()[..4], &[0, 0, 0, 255]);
        assert_eq!(&frame.rgba()[4..8], &[255, 255, 255, 255]);
        let red = CameraPreview::from_yuv420(
            Lens::Front,
            2,
            2,
            &[81, 81, 81, 81, 90, 240],
            std::time::Instant::now(),
        )
        .unwrap();
        assert!(red.rgba()[0] >= 250 && red.rgba()[1] < 3 && red.rgba()[2] < 3);
        let wide =
            CameraPreview::from_yuv420(Lens::Front, 4, 2, &[128; 12], std::time::Instant::now())
                .unwrap();
        assert_eq!(wide.size(), [4, 2]);
        for width in [0, 1, 2, 3, 1920, 1921, u32::MAX] {
            for length in [0, 1, 5, 6, 7, 100] {
                assert_eq!(
                    CameraPreview::from_yuv420(
                        Lens::Front,
                        width,
                        2,
                        &vec![128; length],
                        std::time::Instant::now()
                    )
                    .is_ok(),
                    width == 2 && length == 6
                );
            }
        }
        for size in [[0, 1], [1, 0], [641, 1], [usize::MAX, usize::MAX], [1, 1]] {
            assert!(CameraPreview::new(Lens::Front, size, vec![]).is_err());
        }
    }
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    #[test]
    fn t1457_thumbnail_conversion_does_not_renew_source_age() {
        let published = std::time::Instant::now() - super::super::FRESH_FRAME_AGE;
        let preview =
            CameraPreview::from_yuv420(Lens::Front, 2, 2, &[16, 16, 16, 16, 128, 128], published)
                .unwrap();
        assert!(!preview.fresh(), "T1457 conversion renewed source lifetime");
        assert_eq!(preview.created, published);
    }
}

#[cfg(test)]
mod invalid_publication_tests {
    use super::*;
    #[test]
    fn t1457_future_publication_cannot_extend_freshness() {
        let published = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let preview =
            CameraPreview::from_yuv420(Lens::Front, 2, 2, &[16, 16, 16, 16, 128, 128], published)
                .unwrap();
        assert!(
            !preview.fresh(),
            "T1457 future timestamp extended freshness"
        );
    }
}

#[cfg(test)]
mod quality_tests {
    use super::*;

    #[test]
    fn t1729_preview_preserves_vga_pixel_detail() {
        let mut bytes = vec![128; 640 * 480 * 3 / 2];
        for (index, value) in bytes[..640 * 480].iter_mut().enumerate() {
            *value = if index % 2 == 0 { 16 } else { 235 };
        }
        let preview =
            CameraPreview::from_yuv420(Lens::Front, 640, 480, &bytes, std::time::Instant::now())
                .unwrap();
        assert_eq!(preview.size(), [640, 480]);
        for pair in preview.rgba().chunks_exact(8) {
            assert_eq!(pair, &[0, 0, 0, 255, 255, 255, 255, 255]);
        }
    }

    #[test]
    fn t1729_preview_bounds_preserve_aspect_without_upscaling() {
        for (width, height, expected) in [
            (1920, 1080, [640, 360]),
            (480, 640, [480, 640]),
            (160, 120, [160, 120]),
            (2, 2, [2, 2]),
        ] {
            let bytes = vec![128; (width * height * 3 / 2) as usize];
            let preview = CameraPreview::from_yuv420(
                Lens::Rear,
                width,
                height,
                &bytes,
                std::time::Instant::now(),
            )
            .unwrap();
            assert_eq!(preview.size(), expected);
            assert_eq!(preview.rgba().len(), expected[0] * expected[1] * 4);
            assert!(preview.rgba().len() <= 640 * 640 * 4);
        }
        assert!(CameraPreview::new(Lens::Front, [640, 640], vec![0; 640 * 640 * 4]).is_ok());
        for size in [[641, 1], [1, 641], [0, 640], [usize::MAX, 1]] {
            assert!(CameraPreview::new(Lens::Front, size, vec![]).is_err());
        }
    }
}
