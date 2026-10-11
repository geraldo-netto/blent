//! Audio invitation policy, independent of transport/native implementation.
use super::{AudioOptions, AudioProfile, Direction};
use crate::connection::management::{Audio, Resource};
pub use crate::connection::media::{Connection, Invitation, InvitationResult, Target, Transports};
use anyhow::Result;

impl AudioOptions {
    pub fn target(&self) -> Result<Target<'_>> {
        Target::parse(self.serial.as_deref(), self.usb_peer.as_deref())
    }

    pub fn validate(&self) -> Result<()> {
        self.profile.validate()?;
        self.target()?;
        Ok(())
    }
}

impl AudioProfile {
    /// Projection preserves audio preferences. Clock capability negotiation
    /// remains in the authenticated PCM protocol, independent of selectors.
    pub fn invitation_profile(self) -> Result<(Resource, Audio)> {
        self.validate()?;
        let resource = match self.direction {
            Direction::Microphone => Resource::Microphone,
            Direction::Speakers => Resource::Speakers,
        };
        Ok((
            resource,
            Audio {
                processing: self.processing as u8,
                buffer_ms: self.buffer_ms,
                background: self.background,
            },
        ))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contract_tests;
