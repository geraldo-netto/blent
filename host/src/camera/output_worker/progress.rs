//! A progress deadline is not native settlement. Only the owning task's actual
//! completion can release its file, frame borrow and work credit.
use crate::owned_task::Task;
use anyhow::{Context, Result};
use tokio::{
    sync::watch,
    time::{Duration, Instant},
};

const LIMIT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Configuration,
    Write,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Write => "frame write",
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Pending {
    kind: Kind,
    started: Instant,
}
// This is evidence of an elapsed progress budget, never native quiescence.
struct Expired(Pending);
pub(super) type Sender = watch::Sender<Option<Pending>>;

/// One synchronous native operation. The mutable sender borrow prevents nested
/// spans from clearing another operation's observation. It owns no native data.
#[must_use]
pub(super) struct Span<'a>(&'a mut Sender);
impl<'a> Span<'a> {
    pub(super) fn begin(sender: &'a mut Sender, kind: Kind) -> Self {
        sender.send_replace(Some(Pending {
            kind,
            started: Instant::now(),
        }));
        Self(sender)
    }
}
impl Drop for Span<'_> {
    fn drop(&mut self) {
        self.0.send_replace(None);
    }
}

pub(super) async fn monitor(
    native: &mut Task,
    progress: &mut watch::Receiver<Option<Pending>>,
) -> Result<()> {
    loop {
        let pending = *progress.borrow_and_update();
        tokio::select! { biased;
            result = &mut *native => return joined(result),
            changed = progress.changed() => if changed.is_err() { return joined(native.await); },
            result = expiration(pending) => {
                reject_current(result?, *progress.borrow())?;
            }
        }
    }
}
async fn expiration(pending: Option<Pending>) -> Result<Expired> {
    let Some(active) = pending else {
        return std::future::pending().await;
    };
    let deadline = active
        .started
        .checked_add(LIMIT)
        .context("camera output deadline overflow")?;
    tokio::time::sleep_until(deadline).await;
    Ok(Expired(active))
}
fn joined(result: Result<Result<()>, tokio::task::JoinError>) -> Result<()> {
    result.unwrap_or_else(|error| Err(error.into()))
}
fn reject_current(expired: Expired, current: Option<Pending>) -> Result<()> {
    if current == Some(expired.0) {
        return Err(stalled(expired.0.kind));
    }
    Ok(())
}
fn stalled(kind: Kind) -> anyhow::Error {
    let name = kind.label();
    tracing::warn!(
        operation = name,
        "Camera output progress timed out; original native completion remains unconfirmed"
    );
    anyhow::anyhow!(
        "camera output {name} progress timed out; original native completion remains unconfirmed"
    )
}

#[cfg(test)]
mod tests;
