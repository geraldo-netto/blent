//! Portable camera invitation boundary. Transport selection grants neither
//! connection authority nor Android camera permission. Native adapters own both.
use super::{CameraProfile, Lens};
use crate::connection::{management::Camera, pairing::PeerId};
use anyhow::{ensure, Result};
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target<'a> {
    /// Automatic selection is restricted to the existing ADB adapter.
    Adb(Option<&'a str>),
    /// Explicit certificate identity; never inferred from display selection.
    Usb(PeerId),
}

/// Implemented connection adapters, independent of OS/device types. Availability
/// grants no permission or physical qualification; callers disclose validation
/// separately. Default means no implemented backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Transports {
    pub adb: bool,
    pub usb: bool,
}

impl Transports {
    pub fn available(self) -> bool {
        self.adb || self.usb
    }

    pub fn supports(self, target: &Target<'_>) -> bool {
        match target {
            Target::Adb(_) => self.adb,
            Target::Usb(_) => self.usb,
        }
    }
}

pub type InvitationResult<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Requesting an invitation is independent of capture and OS output devices.
/// Success means the transport accepted the request, not permission or capture.
/// Concrete adapters retain their own authority and cleanup ownership separately.
pub trait Invitation: Send + Sync {
    fn invite<'a>(
        &'a self,
        credential: &'a str,
        profile: &'a CameraProfile,
    ) -> InvitationResult<'a>;
}

/// Pipeline supervision is separate from invitation acceptance. Monitoring stays
/// pending while this original connection is usable; loss returns an error.
/// Close revokes promptly and waits boundedly for cleanup. An error does not
/// prove retirement: adapters retain uncertain native ownership independently.
pub trait Connection: Invitation {
    fn monitor(&self) -> InvitationResult<'_>;
    fn close(&self) -> InvitationResult<'_>;
}

impl CameraProfile {
    pub fn target(&self) -> Result<Target<'_>> {
        match (&self.serial, &self.usb_peer) {
            (serial, None) => Ok(Target::Adb(serial.as_deref())),
            (None, Some(peer)) => Ok(Target::Usb(PeerId::parse(peer.clone())?)),
            _ => anyhow::bail!("camera serial and USB peer are mutually exclusive"),
        }
    }

    /// Checked projection to the existing remote camera schema. Rotation/mirror
    /// remain host decoder transforms; transport selectors never enter the wire.
    pub fn invitation_profile(&self) -> Result<Camera> {
        self.validate()?;
        Ok(Camera {
            width: self.width as u16,
            height: self.height as u16,
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
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid camera token"
    );
    Ok(())
}
