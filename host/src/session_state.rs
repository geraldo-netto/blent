//! Session lifecycle values, independent of task and native implementations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub worker: &'static str,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Failed(Failure),
    Stopped,
    /// Owner dropped; cancellation requested, native retirement not confirmed.
    Cancelled,
}
