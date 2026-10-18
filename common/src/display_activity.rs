//! Display work hints, never media permission or native retirement evidence.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

mod projection;
pub use projection::Projection;

pub const VERSION: u32 = 1;
pub const MAX_REVISION: u64 = i64::MAX as u64;

/// Counters belong to one original authenticated control connection. A new
/// connection must install a fresh consumer owner; equal counters cannot join it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Fields", into = "Fields")]
pub struct Signal {
    revision: u64,
    stopped: u64,
    active: bool,
}
impl Signal {
    pub fn new(revision: u64, stopped: u64, active: bool) -> Result<Self> {
        ensure!(
            (1..=MAX_REVISION).contains(&revision),
            "invalid display activity revision"
        );
        ensure!(stopped <= revision, "future display stop fence");
        Ok(Self {
            revision,
            stopped,
            active,
        })
    }
    pub fn revision(self) -> u64 {
        self.revision
    }
    pub fn stopped(self) -> u64 {
        self.stopped
    }
    pub fn active(self) -> bool {
        self.active
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    version: u32,
    revision: u64,
    stopped: u64,
    active: bool,
}
impl TryFrom<Fields> for Signal {
    type Error = anyhow::Error;
    fn try_from(value: Fields) -> Result<Self> {
        ensure!(
            value.version == VERSION,
            "unsupported display activity version"
        );
        Self::new(value.revision, value.stopped, value.active)
    }
}
impl From<Signal> for Fields {
    fn from(value: Signal) -> Self {
        Self {
            version: VERSION,
            revision: value.revision,
            stopped: value.stopped,
            active: value.active,
        }
    }
}

#[cfg(test)]
mod tests;
