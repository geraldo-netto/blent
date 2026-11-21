//! Daemon liveness observes shared session contracts independently of native cleanup.
use anyhow::{Context, Result};
use blent::session_state::{Failure, State};
use tokio::sync::watch;

pub(super) async fn wait(
    primary: watch::Receiver<State>,
    monitor: watch::Receiver<Option<Failure>>,
    shutdown: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    tokio::select! {
        biased;
        result = shutdown => result,
        result = primary_failed(primary) => result,
        result = monitor_failed(monitor) => result,
    }
}

pub(super) fn session_failure(state: &State) -> Option<Failure> {
    match state {
        State::Running => None,
        State::Failed(failure) => Some(failure.clone()),
        state => Some(Failure {
            worker: "Session",
            message: format!("Unexpected state: {state:?}"),
        }),
    }
}

async fn primary_failed(mut status: watch::Receiver<State>) -> Result<()> {
    loop {
        if let Some(failure) = session_failure(&status.borrow()) {
            anyhow::bail!("Primary {} failed: {}", failure.worker, failure.message);
        }
        status
            .changed()
            .await
            .context("Primary session status closed unexpectedly")?;
    }
}

async fn monitor_failed(mut status: watch::Receiver<Option<Failure>>) -> Result<()> {
    loop {
        if let Some(failure) = status.borrow().as_ref() {
            anyhow::bail!("{} failed: {}", failure.worker, failure.message);
        }
        status
            .changed()
            .await
            .context("Connection monitor ended unexpectedly")?;
    }
}

#[cfg(test)]
pub(crate) mod tests;
