//! Optional center-metered shadow lift for bounded, limited-range YUV420 frames.
//! One decoder owns this scalar state. Each call exclusively borrows a complete
//! unpublished frame; no buffer, reference, callback or native resource is retained.
use super::CameraProfile;
use anyhow::{ensure, Result};

pub struct AutoBrightness {
    width: usize,
    height: usize,
    bytes: usize,
    gamma: f32,
    blend: f32,
}

impl AutoBrightness {
    pub fn new(profile: &CameraProfile) -> Result<Self> {
        let bytes = profile.frame_bytes()?;
        ensure!((5..=30).contains(&profile.fps), "camera FPS must be 5–30");
        Ok(Self {
            width: profile.width as usize,
            height: profile.height as usize,
            bytes,
            gamma: 1.0,
            // Approximately 0.3 seconds to settle 63% of an exposure change.
            blend: 1.0 - (-1.0 / (profile.fps as f32 * 0.3)).exp(),
        })
    }

    pub fn apply(&mut self, frame: &mut [u8]) -> Result<()> {
        ensure!(
            frame.len() == self.bytes,
            "camera brightness frame length mismatch"
        );
        let luma = &mut frame[..self.width * self.height];
        let mean = self.center_mean(luma);
        let target = (0.5f32.ln() / mean.clamp(1.0 / 219.0, 0.5).ln()).clamp(0.45, 1.0);
        self.gamma += (target - self.gamma) * self.blend;
        let table = curve(self.gamma);
        for value in luma {
            *value = table[*value as usize];
        }
        Ok(())
    }

    fn center_mean(&self, luma: &[u8]) -> f32 {
        let mut total = 0u32;
        let mut count = 0u32;
        // Central half of each axis; at most 64 x 64 samples, no face detection.
        for y in (self.height / 4..self.height * 3 / 4).step_by(self.height.div_ceil(128)) {
            for x in (self.width / 4..self.width * 3 / 4).step_by(self.width.div_ceil(128)) {
                total += u32::from(luma[y * self.width + x].clamp(16, 235) - 16);
                count += 1;
            }
        }
        total as f32 / (count as f32 * 219.0)
    }
}

fn curve(gamma: f32) -> [u8; 256] {
    let mut table = std::array::from_fn(|value| value as u8);
    for value in 16..=235 {
        let normalized = (value - 16) as f32 / 219.0;
        table[value] = (16.0 + 219.0 * normalized.powf(gamma)).round() as u8;
    }
    table
}

#[cfg(test)]
mod tests;
