//! One direct daemon restoration is admitted before process cutoff.
use super::{operation::Gate, restore_unlocked, KEYBOARD_OPERATION};
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::{io, sync::OnceLock, time::Duration};

static DIRECT: OnceLock<io::Result<Pool>> = OnceLock::new();

fn pool(storage: &OnceLock<io::Result<Pool>>) -> io::Result<&Pool> {
    storage
        .get_or_init(|| Pool::new(1))
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
}

/// A process-owned reservation for restoring the saved keyboard setting.
/// Reserve before shutdown; consuming it never admits new process resources.
pub struct Restoration {
    admission: Option<Admission>,
    original: Option<Completion>,
}
impl Restoration {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            admission: Some(crate::process_retirement::PROCESS.reserve(pool(&DIRECT)?)?),
            original: None,
        })
    }

    /// Restore and await the original operation, including remote acknowledgement.
    /// Unknown outcomes remain owned; cancellation transfers the same proof to retirement.
    pub async fn run(self) {
        self.restore(&KEYBOARD_OPERATION).await;
    }

    async fn restore(mut self, gate: &Gate) {
        let operation = match gate.acquire().await {
            Ok(operation) => operation,
            Err(error) => {
                tracing::warn!("OSK restoration admission failed: {error}; disk recovery retained");
                return;
            }
        };
        self.original = Some(operation.completion());
        restore_unlocked(gate.commands.as_ref()).await;
        drop(operation);
        while !self.original.as_ref().unwrap().is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
}
impl Drop for Restoration {
    fn drop(&mut self) {
        let admission = self.admission.take().unwrap();
        if let Some(original) = self.original.take() {
            admission.retain(Pending(original));
        }
    }
}

struct Pending(Completion);
impl Observe for Pending {
    fn poll_complete(&mut self) -> bool {
        self.0.is_complete()
    }
}

#[cfg(test)]
mod tests;
