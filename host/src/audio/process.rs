use anyhow::{Context, Result};
use blent_config::audio::{AudioProfile, Direction};
use blent_config::commands::OwnedChild as Child;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::process::Command;

pub(super) type Start<'a> = dyn Fn(&Path, AudioProfile, &str) -> Result<Child> + Send + Sync + 'a;

pub(super) fn spawn(helper: &Path, profile: AudioProfile, token: &str) -> Result<Child> {
    profile.validate()?;
    anyhow::ensure!(
        blent_config::connection::media::valid_credential(token),
        "invalid audio helper credential"
    );
    let direction = match profile.direction {
        Direction::Microphone => "microphone",
        Direction::Speakers => "speakers",
    };
    let mut command = Command::new(helper);
    command.args([
        profile.buffer_ms.to_string(),
        format!("blent_{direction}_{}", &token[..12]),
        direction.to_owned(),
    ]);
    let child = Child::spawn(
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true),
    )
    .context("Start native PipeWire audio adapter")?;
    use std::os::fd::AsRawFd;
    if let Some(input) = &child.stdin {
        // Minimize stale kernel backlog; child continuously drains into an age-bounded ring.
        unsafe {
            libc::fcntl(input.as_raw_fd(), libc::F_SETPIPE_SZ, 4096);
        }
    }
    if let Some(output) = &child.stdout {
        unsafe {
            libc::fcntl(output.as_raw_fd(), libc::F_SETPIPE_SZ, 4096);
        }
    }
    Ok(child)
}
pub(super) async fn retire(child: Child) -> blent_config::commands::Retirement {
    let retired = child.retire(Duration::from_millis(500)).await;
    if !retired.completion.is_complete() {
        tracing::warn!("Audio process retirement incomplete; capacity retained");
    }
    retired
}

pub(super) async fn ready(child: &mut Child) -> Result<()> {
    use tokio::io::AsyncReadExt;
    let mut ready = [0; 6];
    tokio::time::timeout(
        Duration::from_secs(5),
        child
            .stdout
            .as_mut()
            .context("missing adapter status")?
            .read_exact(&mut ready),
    )
    .await??;
    anyhow::ensure!(&ready == b"READY\n", "PipeWire device could not be created");
    Ok(())
}

#[cfg(test)]
mod tests;
