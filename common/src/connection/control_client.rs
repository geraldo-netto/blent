//! Nonblocking presentation port. Implementations own native transport and
//! lifecycle; views, commands and status contain no OS or GUI types.
use super::control_view::{Action, View};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    Ready,
    Busy,
    Closing,
    Closed,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// Latest completed submission, retained across automatic refreshes. An absent
/// matching outcome never proves completion (including timeout/cancellation).
pub struct Outcome {
    pub id: u64,
    pub error: Option<String>,
}
#[derive(Clone)]
pub struct Status {
    pub phase: Phase,
    pub view: Option<View>,
    pub message: Option<String>,
    pub outcome: Option<Outcome>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Closed,
    Exhausted,
}

pub trait Client: Send + Sync {
    fn status(&self) -> Status;
    /// Acceptance queues bounded work only. The owning adapter must revalidate
    /// the action before sending. Returns an ordered, nonwrapping lifetime-local
    /// ID. Status retains the latest completed ID and error independently of
    /// polling. Callers needing each outcome must serialize their submissions.
    fn submit(&self, action: Action) -> Result<u64, SubmitError>;
    /// Logical cancellation is immediate; this must not wait for native I/O.
    fn close(&self);
    /// True requires actual client-owned worker/native retirement, not a terminal
    /// status. Independent broker/media lifetimes are not owned by this handle.
    fn retired(&self) -> bool;
}
