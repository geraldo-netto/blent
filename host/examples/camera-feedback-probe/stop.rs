//! Native scoped cutoff. Acknowledgement grants no native retirement proof.
use super::bridge;
use crate::adb_command::broadcast_accepted;
use anyhow::{ensure, Result};
use blent_config::{camera::stop::Request, commands::AsyncCommandExt};
use std::time::Duration;
use tokio::process::Command;

pub(super) async fn stop_capture(bridge: &bridge::Bridge, token: &str) -> Result<()> {
    let request = Request::new(token, bridge.port()?)?;
    let command = format!("am broadcast -n io.github.geraldo_netto.blent/com.blent.CameraReceiver -a com.blent.CAMERA_STOP --es expected_token {} --ei expected_port {}\n", request.credential(), request.port());
    let result = Command::new(&bridge.adb)
        .args(["-s", &bridge.serial, "shell"])
        .output_input_timeout(Some(command.as_bytes()), Duration::from_secs(5))
        .await?;
    ensure!(
        result.status.success() && broadcast_accepted(&result.stdout),
        "tablet did not acknowledge scoped camera stop"
    );
    Ok(())
}
