//! Bounded immutable frame leases. A retained reader can never be overwritten.
use anyhow::{ensure, Result};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt};
const MAX_BYTES: usize = 1920 * 1080 * 3 / 2;

pub(super) struct Pool {
    sensor_rotation: u8,
    slots: Vec<Arc<Vec<u8>>>,
    discard: Vec<u8>,
    transform: Option<blent_config::camera::transform::Transform>,
    live: Option<Arc<crate::camera_live::Session>>,
    brightness: Option<blent_config::camera::brightness::AutoBrightness>,
}
impl Pool {
    pub(super) fn for_profile(profile: &blent_config::camera::CameraProfile) -> Result<Self> {
        let mut pool = Self::configured(profile.frame_bytes()?)?;
        pool.brightness = Some(blent_config::camera::brightness::AutoBrightness::new(
            profile,
        )?);
        pool.transform = Some(blent_config::camera::transform::Transform::new(profile)?);
        pool.live = Some(crate::camera_live::Session::new(profile.clone()));
        Ok(pool)
    }

    pub(super) fn orient(&mut self, rotation: u8) -> Result<()> {
        ensure!(rotation <= 3, "Invalid camera sensor orientation");
        self.sensor_rotation = rotation;
        Ok(())
    }
    pub(super) fn attach(&mut self, live: Arc<crate::camera_live::Session>) {
        self.live = Some(live);
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
            sensor_rotation: 0,
            slots: (0..slots).map(|_| Arc::new(vec![0; bytes])).collect(),
            discard: vec![0; bytes],
            brightness: None,
            transform: None,
            live: None,
        })
    }
    pub(super) async fn read(
        &mut self,
        input: &mut (impl AsyncRead + Unpin),
    ) -> Result<Option<Arc<Vec<u8>>>> {
        for slot in &mut self.slots {
            if let Some(bytes) = Arc::get_mut(slot) {
                input.read_exact(bytes).await?;
                if let Some(live) = &self.live {
                    let (revision, image) = live.snapshot();
                    self.transform
                        .as_mut()
                        .unwrap()
                        .apply(image.oriented(self.sensor_rotation)?, bytes)?;
                    let brightness = self.brightness.as_mut().unwrap();
                    brightness.update(image.shadows, image.strength)?;
                    brightness.apply(bytes)?;
                    live.applied(revision);
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
