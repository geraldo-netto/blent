//! A single retained worker serializes commands and joins every reported child
//! and collector before interpreting a snapshot or starting another operation.
use super::Control;
use anyhow::{ensure, Result};
use blent_config::{commands::AsyncCommandExt, retirement::Completion};
use std::{io, process::Output, time::Duration};
use tokio::process::Command;

pub(super) struct Attempt {
    pub output: io::Result<Output>,
    pub started: bool,
}

pub(super) async fn execute(control: &Control, args: &[&str], input: Option<&str>) -> Attempt {
    let observer = |proof: Completion| {
        let mut proofs = control.commands.lock().unwrap();
        assert!(
            proofs.len() < 2,
            "one collector and one original child per command"
        );
        proofs.push(proof);
    };
    let output = Command::new(&control.target.adb)
        .args(args)
        .output_input_timeout_observed(input.map(str::as_bytes), Duration::from_secs(5), &observer)
        .await;
    let started = control.commands.lock().unwrap().len() == 2;
    fence(control).await;
    Attempt { output, started }
}

async fn fence(control: &Control) {
    while !control
        .commands
        .lock()
        .unwrap()
        .iter()
        .all(Completion::is_complete)
    {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    control.commands.lock().unwrap().clear();
}

pub(super) async fn list(control: &Control) -> Result<String> {
    let attempt = execute(
        control,
        &["-s", &control.target.serial, "reverse", "--list"],
        None,
    )
    .await;
    let output = attempt.output?;
    ensure!(
        output.status.success(),
        "ADB mapping listing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

pub(super) async fn shell(control: &Control, input: &str) -> Result<()> {
    let attempt = execute(
        control,
        &["-s", &control.target.serial, "shell"],
        Some(input),
    )
    .await;
    let output = attempt.output?;
    ensure!(output.status.success(), "ADB media invitation failed");
    ensure!(
        crate::adb_command::broadcast_accepted(&output.stdout),
        "tablet did not accept media invitation"
    );
    Ok(())
}
