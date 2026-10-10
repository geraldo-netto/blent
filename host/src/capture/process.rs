//! Same-user capture process retirement and bounded child termination.
use anyhow::Result;
use blent_config::commands::{OwnedChild as Child, RetirementPhase};
use std::path::Path;
use tracing::{info, warn};

/// SIGTERM first, then reap. The helper installs a SIGTERM handler and uses
/// it to run `evdi_disconnect`; SIGKILL skips that and leaves the connector
/// attached until the kernel gets around to releasing the fd.
pub(super) async fn terminate(child: Child, what: &str) -> blent_config::commands::Retirement {
    if let Some(pid) = child.id() {
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
    }
    let retired = child.retire(std::time::Duration::from_millis(1500)).await;
    match retired.phase {
        RetirementPhase::Exited => info!("{} exited cleanly", what),
        RetirementPhase::ReapedAfterSignal => warn!("{} required forced retirement", what),
        RetirementPhase::Incomplete => warn!("{} retirement unconfirmed; capacity retained", what),
    }
    retired
}
pub(super) fn ensure_retired(pending: &Option<blent_config::retirement::Completion>) -> Result<()> {
    anyhow::ensure!(
        pending
            .as_ref()
            .is_none_or(|completion| completion.is_complete()),
        "previous capture resource is still retiring"
    );
    Ok(())
}

pub(super) async fn retire_orphan_capture(path: &Path) -> Result<()> {
    use blent_config::linux::processes;
    let selected: Vec<_> = processes::same_user_processes()?
        .into_iter()
        .filter(|process| process.capture_role(path).is_some())
        .collect();
    let retired = processes::retire(
        &selected,
        std::time::Duration::from_millis(1500),
        std::time::Duration::from_millis(500),
    )
    .await?;
    if retired > 0 {
        warn!(
            "Retired {} stray capture process(es) before starting",
            retired
        );
    }
    Ok(())
}
