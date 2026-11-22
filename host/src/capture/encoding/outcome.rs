//! Reader work errors are recoverable only after original retirement settles.
use anyhow::Result;
use std::fmt;

#[derive(Debug)]
pub(super) struct WorkFailure(anyhow::Error);
impl fmt::Display for WorkFailure {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, output)
    }
}
impl std::error::Error for WorkFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

pub(super) fn observe(outcome: Result<Result<()>, tokio::task::JoinError>) -> Result<bool> {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("Encoder finished");
            Ok(true)
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "Encoder reader failed; restart requires original retirement");
            Err(WorkFailure(error).into())
        }
        Err(error) if error.is_cancelled() => Ok(false),
        Err(error) => {
            tracing::error!(%error, "Encoder reader panicked");
            Err(error.into())
        }
    }
}
