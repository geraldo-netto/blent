//! Independent media transport contracts. Selection grants no connection authority,
//! remote consent, native-device ownership or physical qualification.
use super::pairing::PeerId;
use anyhow::Result;
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
/// Success means the transport accepted the request, not permission or media delivery.
/// Concrete adapters retain their own authority and cleanup ownership separately.
pub trait Invitation<P>: Send + Sync {
    fn invite<'a>(&'a self, credential: &'a str, profile: &'a P) -> InvitationResult<'a>;
}

/// Pipeline supervision is separate from invitation acceptance. Monitoring stays
/// pending while this original connection is usable; loss returns an error.
/// Close revokes promptly and waits boundedly for cleanup. An error does not
/// prove retirement: adapters retain uncertain native ownership independently.
pub trait Connection<P>: Invitation<P> {
    fn monitor(&self) -> InvitationResult<'_>;
    fn close(&self) -> InvitationResult<'_>;
}

impl<'a> Target<'a> {
    /// USB requires one full explicit certificate identity. Never infer an ADB
    /// serial, active display peer, or recently observed connection as fallback.
    pub fn parse(serial: Option<&'a str>, usb_peer: Option<&str>) -> Result<Self> {
        match (serial, usb_peer) {
            (serial, None) => Ok(Self::Adb(serial)),
            (None, Some(peer)) => Ok(Self::Usb(PeerId::parse(peer.to_owned())?)),
            _ => anyhow::bail!("serial and USB peer are mutually exclusive"),
        }
    }
}
