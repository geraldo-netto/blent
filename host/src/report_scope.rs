//! Publication permission is distinct from native-resource retirement.
//! One workflow owns one current permission. Cutoff waits for in-flight short
//! publications, then permanently seals that exact permission. Retained old
//! permits own metadata only; they cannot become current again after resume.
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Permit(Arc<Mutex<bool>>);
impl Permit {
    /// Synchronous publication only. Do not await or re-enter the related owner.
    /// Native resource ownership/cleanup must be retained independently.
    pub(crate) fn publish(&self, action: impl FnOnce()) {
        let active = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *active {
            action();
        }
    }
}

/// Workflow-owned cutoff authority. Resource budgets remain with native owners.
/// Stop/Drop seals the original permission even if callback clones survive.
/// Conditional progress requires publication read guards/callbacks to quiesce;
/// resource retirement is never inferred from sealing this metadata.
#[derive(Default)]
pub struct Owner(Mutex<Option<Permit>>);
impl Owner {
    pub fn begin(&self) {
        let mut current = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        seal(current.take());
        *current = Some(Permit(Arc::new(Mutex::new(true))));
    }
    pub fn permit(&self) -> Result<Permit> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .context("No current report publication scope")
    }
    pub fn revoke(&self) {
        let mut current = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        seal(current.take());
    }
}
fn seal(original: Option<Permit>) {
    if let Some(original) = original {
        *original
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = false;
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.revoke();
    }
}

#[cfg(test)]
mod tests;
