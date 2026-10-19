//! TLS and authenticated protocol negotiation budget, independent of adapters.
use anyhow::{ensure, Result};
use std::time::Duration;

/// One whole-operation deadline. Expiry never proves native resource retirement.
/// This immutable scalar retains no allocation or borrowed input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeout(Duration);
impl Timeout {
    pub fn new(duration: Duration) -> Result<Self> {
        ensure!(
            !duration.is_zero() && duration <= Duration::from_secs(60),
            "invalid TLS setup timeout"
        );
        Ok(Self(duration))
    }
    pub fn duration(self) -> Duration {
        self.0
    }
}
impl Default for Timeout {
    fn default() -> Self {
        Self(Duration::from_secs(10))
    }
}

#[cfg(test)]
mod tests;
