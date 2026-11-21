//! Shared local launch-path adapter for desktop controls and daemon discovery.
use super::client::Launch;
use anyhow::{Context, Result};
use std::{ffi::OsString, path::PathBuf};

impl Launch {
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
