//! Portable camera invitation boundary. Transport selection grants neither
//! connection authority nor Android camera permission. Native adapters own both.
use super::{CameraProfile, Lens};
use crate::connection::management::Camera;
use anyhow::{ensure, Result};

pub use crate::connection::media::{Connection, Invitation, InvitationResult, Target, Transports};

impl CameraProfile {
    pub fn target(&self) -> Result<Target<'_>> {
        Target::parse(self.serial.as_deref(), self.usb_peer.as_deref())
    }

    /// Native capture profile, distinct from exported webcam dimensions. The
    /// 180p output uses 360p capture and host downscaling; Camera2 still requires
    /// an advertised native size. Rotation/mirror remain host transforms.
    pub fn invitation_profile(&self) -> Result<Camera> {
        self.validate()?;
        let (width, height) = match (self.width, self.height) {
            (320, 180) => (640, 360),
            dimensions => dimensions,
        };
        Ok(Camera {
            width: width as u16,
            height: height as u16,
            fps: self.fps as u8,
            bitrate: self.bitrate as u16,
            min_bitrate: self.min_bitrate as u16,
            freshness_ms: self.freshness_ms as u16,
            lens: match self.lens {
                Lens::Front => 0,
                Lens::Rear => 1,
            },
            background: self.background,
            adaptive: self.adaptive_bitrate,
        })
    }
}

/// Camera authentication keeps the existing 64-character lowercase hex format.
/// Credentials deliberately remain outside Debug/serialization of profiles.
pub fn validate_credential(value: &str) -> Result<()> {
    ensure!(
        crate::connection::media::valid_credential(value),
        "invalid camera token"
    );
    Ok(())
}
