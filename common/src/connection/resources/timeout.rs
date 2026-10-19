//! Immutable admission policy, independent of transports and native ownership.
use anyhow::{ensure, Result};

/// One relative admission budget. Expiry cancels a request; it never proves
/// native retirement. This value retains no resources or borrowed input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionTimeout(i64);
impl AdmissionTimeout {
    pub fn new(milliseconds: i64) -> Result<Self> {
        ensure!(
            (1..=600_000).contains(&milliseconds),
            "invalid resource deadline"
        );
        Ok(Self(milliseconds))
    }
    pub fn milliseconds(self) -> i64 {
        self.0
    }
}
impl Default for AdmissionTimeout {
    fn default() -> Self {
        Self(120_000)
    }
}

#[cfg(test)]
mod tests;
