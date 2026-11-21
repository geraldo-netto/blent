//! Shared local launch-path adapter for desktop controls and daemon discovery.
use super::client::Launch;
use anyhow::{Context, Result};
use std::{ffi::OsString, path::PathBuf};

impl Launch {
    pub(super) fn command(&self) -> std::process::Command {
        let manager = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .is_some_and(|runtime| runtime.join("systemd/private").exists());
        self.command_with_manager(manager)
    }
    fn command_with_manager(&self, manager: bool) -> std::process::Command {
        if !manager {
            return std::process::Command::new(&self.executable);
        }
        // Move before exec: the elected shared broker and its native children
        // outlive a requesting daemon/GUI, in the same scope and environment.
        // A reachable manager is required once selected; never silently fall
        // back to putting shared native owners in the client's retirement group.
        let mut command = std::process::Command::new("/usr/bin/systemd-run");
        command
            .args([
                "--user",
                "--scope",
                "--quiet",
                "--property=KillMode=mixed",
                "--property=SendSIGKILL=no",
                "--property=TimeoutStopSec=infinity",
                "--",
            ])
            .arg(&self.executable);
        command
    }
    /// Resolves configuration only; never scans directories or opens USB/token data.
    pub fn from_environment() -> Result<Self> {
        Self::from_paths(
            std::env::current_exe().context("locate Blent executable")?,
            std::env::var_os("XDG_RUNTIME_DIR"),
            std::env::var_os("XDG_STATE_HOME"),
            std::env::var_os("HOME"),
        )
    }
    // Source builds, tarballs and AppImages carry both native siblings here.
    pub fn from_paths(
        executable: PathBuf,
        runtime: Option<OsString>,
        state: Option<OsString>,
        home: Option<OsString>,
    ) -> Result<Self> {
        let executable = absolute(Some(executable.into_os_string()), "Blent executable")?;
        let directory = executable
            .parent()
            .context("Blent executable has no directory")?;
        let state = match state {
            Some(state) => absolute(Some(state), "XDG_STATE_HOME")?,
            None => absolute(home, "HOME")?.join(".local/state"),
        };
        Ok(Self {
            executable: directory.join("blent-usb-broker"),
            helper: directory.join("blent-usb-io"),
            runtime: absolute(runtime, "XDG_RUNTIME_DIR")?.join("blent-usb"),
            state: state.join("blent/connection"),
        })
    }
}
fn absolute(value: Option<OsString>, name: &str) -> Result<PathBuf> {
    value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .with_context(|| format!("{name} must be an absolute path"))
}

#[cfg(test)]
mod tests;
