//! T877 local broker control contract. IDs are issued by the current broker;
//! USB metadata is a display hint, never authentication or approval authority.
//! Native handles, paths, processes and UI types do not cross this boundary.
use super::pairing::{PeerId, Ticket};
mod codec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Bounded snapshot records terminate with End carrying the same revision.
    Inspect,
    Connect {
        revision: u64,
        target: u64,
    },
    /// Both the connection ID and exact ticket must still be current.
    Decide {
        connection: u64,
        ticket: Ticket,
        allow: bool,
        remember: bool,
    },
    Disconnect {
        connection: u64,
        ticket: Option<Ticket>,
    },
    Revoke {
        peer: PeerId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Discovering = 0,
    Pending = 1,
    Approved = 2,
    Authenticated = 3,
    Closing = 4,
    Quarantined = 5,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// Starts a snapshot; local is the computer certificate fingerprint.
    Begin {
        revision: u64,
        local: PeerId,
    },
    /// Target is opaque. Even an accessory VID/PID proves no peer identity.
    Device {
        revision: u64,
        target: u64,
        vendor: u16,
        product: u16,
        accessory: bool,
    },
    Link {
        revision: u64,
        connection: u64,
        ticket: Option<Ticket>,
        state: State,
        trusted: bool,
    },
    Saved {
        revision: u64,
        peer: PeerId,
    },
    End {
        revision: u64,
    },
}

#[cfg(test)]
mod tests;
