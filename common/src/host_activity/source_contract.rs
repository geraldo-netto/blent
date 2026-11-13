//! Independent source creation/publication boundary, without OS or runtime types.
use super::*;
use std::{future::Future, pin::Pin};

/// Retain through actual creator/native destruction and join/reap, including
/// errors, cancellation and dropped results. It is never authority to acquire media.
pub use super::source_lifetime::SourceLifetime;
/// Each replacement must have an epoch strictly greater than its predecessor.
/// The caller retires the original source and its complete native lifetime before
/// invoking the factory again. No observations transfer across that boundary.
pub struct StartedSource {
    pub epoch: u64,
    pub source: Box<dyn Source>,
}
/// A known observer could not establish its initial evidence. No capability or
/// epoch is admitted. The caller may retry only after this attempt's exact native
/// completion, with the same bounded backoff and stop control as replacements.
#[derive(Debug)]
pub struct RetrySourceCreation;
impl std::fmt::Display for RetrySourceCreation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("initial native activity evidence unavailable; retry after settlement")
    }
}
impl std::error::Error for RetrySourceCreation {}
pub type SourceCreation = Pin<Box<dyn Future<Output = Result<StartedSource>> + Send>>;
pub trait SourceFactory: Send + Sync {
    fn create(&self, settings: SourceSettings, lifetime: SourceLifetime) -> SourceCreation;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    Pending,
    Observed {
        capabilities: Capabilities,
        snapshot: ActivitySnapshot,
    },
    Failed,
    Stopped,
}
