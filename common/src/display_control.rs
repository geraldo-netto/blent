//! Per-tablet display intent/status protocol, independent of native slots,
//! process IDs, IPC mechanisms and UI. Replies acknowledge intent; only absence
//! of the original lease in the same owner's complete snapshot means retirement.
use crate::{connection::pairing::PeerId, tablets::Identity};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

pub const MAX_FRAME: usize = 32_768;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "transport", content = "identity", rename_all = "snake_case")]
pub enum Target {
    Adb(String),
    Usb(String),
}

impl Target {
    pub fn from_identity(identity: &Identity) -> Self {
        match identity {
            Identity::Adb(serial) => Self::Adb(serial.clone()),
            Identity::UsbPeer(peer) => Self::Usb(peer.as_str().to_owned()),
        }
    }
    pub fn identity(&self) -> Result<Identity> {
        let identity = match self {
            Self::Adb(serial) => Identity::Adb(serial.clone()),
            Self::Usb(peer) => Identity::UsbPeer(PeerId::parse(peer.clone())?),
        };
        crate::workspace::validate_identity(&identity)?;
        Ok(identity)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lease {
    pub slot: u32,
    pub generation: u64,
}

impl Lease {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.slot < 4 && self.generation != 0,
            "invalid display lease"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Starting,
    Assigned,
    Stopping,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub target: Target,
    pub lease: Lease,
    pub phase: Phase,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub owner: String,
    pub assignments: Vec<Assignment>,
}

impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            crate::connection::media::valid_credential(&self.owner),
            "invalid display owner"
        );
        ensure!(
            self.assignments.len() <= 4,
            "display snapshot capacity exceeded"
        );
        let mut slots = std::collections::BTreeSet::new();
        let mut identities = Vec::new();
        for assignment in &self.assignments {
            assignment.lease.validate()?;
            let identity = assignment.target.identity()?;
            ensure!(!identities.contains(&identity), "duplicate display target");
            ensure!(
                slots.insert(assignment.lease.slot),
                "duplicate display slot"
            );
            identities.push(identity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Inspect {},
    Set {
        owner: String,
        target: Target,
        enabled: bool,
        expected: Option<Lease>,
    },
}

impl Command {
    pub fn validate(&self) -> Result<()> {
        let Self::Set {
            owner,
            target,
            enabled,
            expected,
        } = self
        else {
            return Ok(());
        };
        ensure!(
            crate::connection::media::valid_credential(owner),
            "invalid display owner"
        );
        target.identity()?;
        ensure!(
            *enabled == expected.is_none(),
            "Stop requires an original lease; Start requires no lease"
        );
        if let Some(lease) = expected {
            lease.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub snapshot: Snapshot,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests;
