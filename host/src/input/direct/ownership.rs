//! Bounded original workers remain sponsored through normal process return.
use anyhow::{ensure, Result};
use blent_config::retirement::{Admission, Pool};
use std::sync::{Mutex, OnceLock};

struct Owners {
    pool: Pool,
    closed: Mutex<bool>,
}
static OWNERS: OnceLock<std::io::Result<Owners>> = OnceLock::new();
impl Owners {
    fn new(capacity: usize) -> std::io::Result<Self> {
        Ok(Self {
            pool: Pool::new(capacity)?,
            closed: Mutex::new(false),
        })
    }
    fn shared() -> Result<&'static Self> {
        OWNERS
            .get_or_init(|| {
                Self::new(blent_config::retirement_limits::configured(
                    "BLENT_INPUT_WORKER_LIMIT",
                    16,
                    4096,
                )?)
            })
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))
    }
    fn reserve(&self) -> Result<Admission> {
        let closed = self.closed.lock().unwrap();
        ensure!(!*closed, "Input process is closing");
        Ok(self.pool.reserve()?)
    }
    fn shutdown(&self) {
        *self.closed.lock().unwrap() = true;
        while self.pool.active() != 0 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
pub(super) fn reserve() -> Result<Admission> {
    Owners::shared()?.reserve()
}

/// Drop only at process entry, after producers/runtime have been destroyed.
/// Forced OS termination cannot execute this normal-return fence.
pub struct ProcessExit;
impl Drop for ProcessExit {
    fn drop(&mut self) {
        if let Some(Ok(owners)) = OWNERS.get() {
            owners.shutdown();
        }
    }
}

#[cfg(test)]
mod tests;
