//! Shared native ADB output; existing command owners retain children/collectors.
use anyhow::{ensure, Result};
use blent_config::commands::AsyncCommandExt;
use std::{path::Path, time::Duration};
use tokio::process::Command;

pub(crate) async fn output(adb: &Path, args: &[&str]) -> Result<String> {
    output_observed(adb, args, &|_| {}).await
}

pub(crate) async fn output_observed(
    adb: &Path,
    args: &[&str],
    observer: &(dyn Fn(blent_config::retirement::Completion) + Send + Sync),
) -> Result<String> {
    let result = Command::new(adb)
        .args(args)
        .output_input_timeout_observed(None, Duration::from_secs(5), observer)
        .await?;
    ensure!(
        result.status.success(),
        "ADB command failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}
