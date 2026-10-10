//! T871: local registration IPC, independent of operating-system handles.
//! The authenticated adapter supplies endpoint capabilities separately. Packet
//! contents never prove descriptor ownership or grant a remote media lease.
//! Pairing/UI commands are a future explicit protocol extension, not registration.
use super::{
    management::{Credential, Profile, Resource},
    pairing::PeerId,
    routes::RouteId,
    wire::MAX_COUNTER,
};
use anyhow::{ensure, Result};
mod codec;

pub const HEADER_BYTES: usize = 16;
pub const MAX_PACKET: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Queued = 0,
    Active = 1,
    Closing = 2,
    Retired = 3,
    Failed = 4,
}
impl TryFrom<u8> for State {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Queued,
            1 => Self::Active,
            2 => Self::Closing,
            3 => Self::Retired,
            4 => Self::Failed,
            _ => anyhow::bail!("unknown registration state"),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Rejection {
    Invalid = 0,
    Unavailable = 1,
    Busy = 2,
    Stale = 3,
    Denied = 4,
    Failed = 5,
}
impl TryFrom<u8> for Rejection {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Invalid,
            1 => Self::Unavailable,
            2 => Self::Busy,
            3 => Self::Stale,
            4 => Self::Denied,
            5 => Self::Failed,
            _ => anyhow::bail!("unknown registration rejection"),
        })
    }
}

/// Request IDs correlate local commands only. They are never management request
/// IDs or channel generations. Registered reserves local ownership, not media
/// authority. Status uses the initiating request: the registration request for
/// normal lifecycle events, or the unregister request for its closing/retirement.
/// Retired means actual endpoint retirement; Failed alone does not release it.
/// The client retains RouteId to disambiguate delayed events from either request.
#[derive(Clone, PartialEq, Eq)]
pub enum Message {
    Register {
        request: u64,
        peer: PeerId,
        resource: Resource,
        credential: Credential,
        profile: Profile,
    },
    Unregister {
        request: u64,
        route: RouteId,
    },
    Registered {
        request: u64,
        route: RouteId,
    },
    Status {
        request: u64,
        route: RouteId,
        state: State,
    },
    Rejected {
        request: u64,
        reason: Rejection,
    },
}
impl Message {
    pub fn request(&self) -> u64 {
        match self {
            Self::Register { request, .. } => *request,
            Self::Unregister { request, .. } => *request,
            Self::Registered { request, .. } => *request,
            Self::Status { request, .. } => *request,
            Self::Rejected { request, .. } => *request,
        }
    }
    /// Native adapters must require exactly this many handles, close extras on
    /// rejection, and authenticate ownership independently. Display order is
    /// video then control; other resources have exactly one byte-stream endpoint.
    pub fn descriptor_count(&self) -> usize {
        match self {
            Self::Register {
                resource: Resource::DisplaySession,
                ..
            } => 2,
            Self::Register { .. } => 1,
            _ => 0,
        }
    }
}
fn counter(value: u64) -> Result<()> {
    ensure!(
        (1..=MAX_COUNTER).contains(&value),
        "invalid local registration counter"
    );
    Ok(())
}
