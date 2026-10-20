//! Probe-local timestamp storage; no native handles or frame borrows.
use anyhow::{ensure, Context, Result};
use std::time::Duration;

pub(super) const DEFAULT_CAPACITY: usize = 8192;
pub(super) const MAX_CAPACITY: usize = 1_048_576;

pub(super) struct Samples {
    timestamps: Vec<u64>,
    capacity: usize,
}

impl Samples {
    pub(super) fn new(capacity: usize) -> Result<Self> {
        ensure!(
            (1..=MAX_CAPACITY).contains(&capacity),
            "probe sample capacity must be 1–1048576"
        );
        let mut timestamps = Vec::new();
        timestamps
            .try_reserve_exact(capacity)
            .context("reserve probe timestamps")?;
        Ok(Self {
            timestamps,
            capacity,
        })
    }

    pub(super) fn record(&mut self, elapsed: Duration) -> Result<()> {
        ensure!(
            self.timestamps.len() < self.capacity,
            "probe sample capacity exhausted; increase --max-samples"
        );
        let milliseconds = u64::try_from(elapsed.as_millis())
            .context("probe timestamp exceeds u64 milliseconds")?;
        self.timestamps.push(milliseconds);
        Ok(())
    }

    pub(super) fn count(&self) -> usize {
        self.timestamps.len()
    }

    pub(super) fn first(&self) -> Option<u64> {
        self.timestamps.first().copied()
    }

    pub(super) fn timestamps(&self) -> &[u64] {
        &self.timestamps
    }
}
