//! Linux placement for applications whose lifetime is independent of a caller.
//! Command owns copied argv; the caller retains its original spawn/reaper policy.
//! No process is started here, and no native completion is inferred from a scope.
use std::{ffi::OsStr, path::PathBuf, process::Command};

pub fn command(program: &OsStr) -> Command {
    let manager = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .is_some_and(|runtime| runtime.join("systemd/private").exists());
    command_with_manager(program, manager)
}

pub(crate) fn command_with_manager(program: &OsStr, manager: bool) -> Command {
    if !manager {
        return Command::new(program);
    }
    // Move before exec; preserve the selected executable and environment.
    // A selected manager failure must never silently fall back into the
    // caller's retirement group. Native owners retain their shutdown fences.
    let mut command = Command::new("/usr/bin/systemd-run");
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
        .arg(program);
    command
}
