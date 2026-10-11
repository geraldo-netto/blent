//! T913: activity preferences are not native acquisition or retirement proof.
//! OS adapters supply explicit observations; no inactivity or transport-loss
//! inference. One independent Gate per resource retains actual cleanup proof.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
mod gate;
mod policy;
pub use gate::{Gate, Plan, Ticket};
pub use policy::Policy;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pause_on_lock: bool,
    pub pause_on_display_off: bool,
    pub pause_without_viewer: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pause_on_lock: true,
            pause_on_display_off: true,
            pause_without_viewer: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub power: bool,
    pub lock: bool,
    pub display: bool,
}
impl Capabilities {
    pub fn from_mask(mask: u8) -> Result<Self> {
        ensure!(mask <= 7, "unsupported host activity capability bits");
        Ok(Self {
            power: mask & 1 != 0,
            lock: mask & 2 != 0,
            display: mask & 4 != 0,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    Running,
    Suspending,
    Unavailable,
}
/// None on a supported field means unknown; on an unsupported field it is the
/// only accepted value. Explicit unavailable power is not USB unplug evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostState {
    pub power: Option<Power>,
    pub locked: Option<bool>,
    pub display_on: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub epoch: u64,
    pub revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub epoch: u64,
    pub revision: u64,
    pub state: HostState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    PowerUnknown,
    Suspending,
    Unavailable,
    Locked,
    LockUnknown,
    DisplayOff,
    DisplayUnknown,
    NoViewer,
}
/// Desired work only. Native acquisition additionally requires Gate reservation
/// and a fresh exact original-resource authority check before dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Demand {
    pub stamp: Stamp,
    pub display: bool,
    pub auxiliary: bool,
    pub unchanged: bool,
    pub reason: Option<Reason>,
}
/// Prompt OS adapter; poll must not perform blocking native I/O. Source lifetime
/// and actual worker/backend retirement remain separately owned by the adapter.
pub trait Source: Send {
    fn capabilities(&self) -> Capabilities;
    fn poll(&mut self) -> Result<Option<Event>>;
    fn stop(&mut self);
    fn retired(&self) -> bool;
}
/// Trusted boundary checks the exact original authenticated resource lifetime;
/// a parsed identifier or current UI selection alone cannot implement this proof.
pub trait Authority {
    fn current(&self, lifetime: u64) -> bool;
}
#[cfg(test)]
mod tests;
