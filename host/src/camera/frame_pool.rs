//! Bounded immutable frame leases. A retained reader can never be overwritten.
use anyhow::{ensure, Result};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt};
const MAX_BYTES: usize = 1920 * 1080 * 3 / 2;

pub(super) struct Pool {
    slots: Vec<Arc<Vec<u8>>>,
    discard: Vec<u8>,
    brightness: Option<blent_config::camera::brightness::AutoBrightness>,
}
impl Pool {
    pub(super) fn for_profile(profile: &blent_config::camera::CameraProfile) -> Result<Self> {
        let mut pool = Self::configured(profile.frame_bytes()?)?;
        pool.brightness = profile
            .auto_brightness
            .then(|| blent_config::camera::brightness::AutoBrightness::new(profile))
            .transpose()?;
        Ok(pool)
    }

    pub(super) fn configured(bytes: usize) -> Result<Self> {
        let slots = blent_config::retirement_limits::configured("BLENT_CAMERA_FRAME_SLOTS", 3, 16)?;
        Self::new(bytes, slots)
    }
    pub(super) fn new(bytes: usize, slots: usize) -> Result<Self> {
        ensure!(
            (1..=MAX_BYTES).contains(&bytes),
            "Invalid camera frame length"
        );
        ensure!(
            (2..=16).contains(&slots),
            "Camera frame slots must be 2..16"
        );
        Ok(Self {
            slots: (0..slots).map(|_| Arc::new(vec![0; bytes])).collect(),
            discard: vec![0; bytes],
            brightness: None,
        })
    }
    pub(super) async fn read(
        &mut self,
        input: &mut (impl AsyncRead + Unpin),
    ) -> Result<Option<Arc<Vec<u8>>>> {
        for slot in &mut self.slots {
            if let Some(bytes) = Arc::get_mut(slot) {
                input.read_exact(bytes).await?;
                if let Some(brightness) = &mut self.brightness {
                    brightness.apply(bytes)?;
                }
                return Ok(Some(slot.clone()));
            }
        }
        // Drain one whole frame to preserve framing/freshness without an
        // unbounded allocation or waiting for a stuck preview/output reader.
        input.read_exact(&mut self.discard).await?;
        Ok(None)
    }
}

#[cfg(test)]
#[path = "frame_pool/tests.rs"]
mod tests;
