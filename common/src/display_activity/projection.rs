//! One bounded projection owner per original authenticated control connection.
use super::{Signal, MAX_REVISION};
use crate::host_activity::{ActivityTarget, Publication, WorkObservation};
use anyhow::{ensure, Result};

#[derive(Default)]
pub struct Projection {
    original: Option<WorkObservation>,
    last: Option<Signal>,
    failed: bool,
}
impl Projection {
    /// A changed stop fence must be published even when latest demand is Active.
    /// None source retains the independently specified unsupported-source policy;
    /// composition decides whether to announce this protocol at all.
    pub fn observe(
        &mut self,
        publication: Option<Publication>,
        source_open: bool,
    ) -> Result<Option<Signal>> {
        ensure!(!self.failed, "display activity projection exhausted");
        let next = WorkObservation::begin(ActivityTarget::Display, publication, source_open);
        let retired = self.original.is_some_and(|original| {
            !original.permits(ActivityTarget::Display, publication, source_open)
        });
        if self.unchanged(next.is_some(), retired) {
            return Ok(None);
        }
        match self.advance(next.is_some(), retired) {
            Ok(signal) => {
                self.original = next;
                self.last = Some(signal);
                Ok(Some(signal))
            }
            Err(error) => {
                self.failed = true;
                self.original = None;
                Err(error)
            }
        }
    }
    fn unchanged(&self, active: bool, retired: bool) -> bool {
        self.last.is_some_and(|last| last.active == active) && !retired
    }
    fn advance(&self, active: bool, retired: bool) -> Result<Signal> {
        let revision = self
            .last
            .map_or(0, |last| last.revision)
            .checked_add(1)
            .filter(|next| *next <= MAX_REVISION)
            .ok_or_else(|| anyhow::anyhow!("display activity revision exhausted"))?;
        let stopped = self
            .last
            .map_or(0, |last| last.stopped)
            .checked_add(u64::from(retired))
            .ok_or_else(|| anyhow::anyhow!("display activity stop fence exhausted"))?;
        Signal::new(revision, stopped, active)
    }
}

#[cfg(test)]
mod tests;
