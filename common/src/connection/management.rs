//! T850: bounded typed management records. Parsing grants no identity, endpoint
//! authority or media consent. Credentials deliberately have no Debug output.
use anyhow::{ensure, Result};
mod codec;
mod decoder;
pub use decoder::Decoder;
pub const HEADER_BYTES: usize = 28;
pub const MAX_RECORD: usize = HEADER_BYTES + 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Resource {
    DisplaySession = 1,
    Camera = 2,
    Microphone = 3,
    Speakers = 4,
}
impl TryFrom<u8> for Resource {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            1 => Self::DisplaySession,
            2 => Self::Camera,
            3 => Self::Microphone,
            4 => Self::Speakers,
            _ => anyhow::bail!("unknown resource"),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub resource: Resource,
    pub request: u64,
    pub generation: u64,
}
impl Key {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=super::wire::MAX_COUNTER).contains(&self.request),
            "invalid request"
        );
        ensure!(
            (1..=super::wire::MAX_COUNTER).contains(&self.generation),
            "invalid generation"
        );
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct Credential([u8; 32]);
impl Credential {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Camera {
    pub width: u16,
    pub height: u16,
    pub fps: u8,
    pub bitrate: u16,
    pub min_bitrate: u16,
    pub freshness_ms: u16,
    pub lens: u8,
    pub background: bool,
    pub adaptive: bool,
}
impl Camera {
    pub fn validate(&self) -> Result<()> {
        self.validate_image()?;
        ensure!(
            (256..=20000).contains(&self.bitrate) && (256..=20000).contains(&self.min_bitrate),
            "invalid camera bitrate"
        );
        ensure!(
            (50..=2000).contains(&self.freshness_ms) && self.lens <= 1,
            "invalid camera freshness/lens"
        );
        Ok(())
    }
    fn validate_image(&self) -> Result<()> {
        ensure!(
            (160..=1920).contains(&self.width) && self.width % 2 == 0,
            "invalid camera width"
        );
        ensure!(
            (120..=1080).contains(&self.height) && self.height % 2 == 0,
            "invalid camera height"
        );
        ensure!((5..=30).contains(&self.fps), "invalid camera fps");
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audio {
    pub processing: u8,
    pub buffer_ms: u16,
    pub background: bool,
}
impl Audio {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=2).contains(&self.processing),
            "invalid audio processing"
        );
        ensure!(
            (20..=200).contains(&self.buffer_ms) && self.buffer_ms % 10 == 0,
            "invalid audio buffer"
        );
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Display,
    Camera(Camera),
    Audio(Audio),
}
impl Profile {
    fn validate(&self, resource: Resource) -> Result<()> {
        match (self, resource) {
            (Self::Display, Resource::DisplaySession) => Ok(()),
            (Self::Camera(value), Resource::Camera) => value.validate(),
            (Self::Audio(value), Resource::Microphone | Resource::Speakers) => value.validate(),
            _ => anyhow::bail!("profile does not match resource"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Outcome {
    Admitted = 0,
    Unavailable = 1,
    PermissionRequired = 2,
    PermissionDenied = 3,
    Busy = 4,
    Revoked = 5,
    Failed = 6,
}
impl TryFrom<u8> for Outcome {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Admitted,
            1 => Self::Unavailable,
            2 => Self::PermissionRequired,
            3 => Self::PermissionDenied,
            4 => Self::Busy,
            5 => Self::Revoked,
            6 => Self::Failed,
            _ => anyhow::bail!("unknown outcome"),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Reason {
    Requested = 0,
    ConnectionLost = 1,
    WorkerFailed = 2,
    TimedOut = 3,
    PermissionRevoked = 4,
}
impl TryFrom<u8> for Reason {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Requested,
            1 => Self::ConnectionLost,
            2 => Self::WorkerFailed,
            3 => Self::TimedOut,
            4 => Self::PermissionRevoked,
            _ => anyhow::bail!("unknown revoke reason"),
        })
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum Message {
    Capabilities(u8),
    Offer(Key, Credential, Profile),
    Prepared(Key),
    Commit(Key),
    Result(Key, Outcome),
    Revoke(Key, Reason),
}
