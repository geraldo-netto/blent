//! Linux service bootstrap. systemd-run admits this process into a separate
//! scope before dispatch. That scope owns any shared server forked by ADB;
//! Blent owns and waits only for the original start-server client. No server
//! stop, PID adoption, pipe collector, or runtime retirement debt is introduced.
use anyhow::{ensure, Context, Result};
use std::{ffi::OsString, process::Command};

pub fn dispatch(mut args: impl Iterator<Item = OsString>) -> Option<Result<()>> {
    if args.next()?.to_str() != Some("--blent-start-adb-server") {
        return None;
    }
    Some(start(args.next().is_none(), Command::new("adb")))
}

fn start(valid: bool, mut command: Command) -> Result<()> {
    ensure!(
        valid,
        "ADB service bootstrap accepts no additional arguments"
    );
    // Override the service's connect-only socket for this independent scope.
    // PATH retains the same packaged/native ADB selected by the daemon launcher.
    let status = match command.args(["-L", "tcp:5037", "start-server"]).status() {
        Ok(status) => status,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("ADB unavailable; direct USB remains available");
            return Ok(());
        }
        Err(error) => {
            return Err(error).context("starting shared ADB server in its independent scope")
        }
    };
    ensure!(
        status.success(),
        "shared ADB server startup failed: {status}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn t1465_dispatch_rejects_other_and_extra_arguments_without_starting_adb() {
        assert!(dispatch(std::iter::empty()).is_none());
        assert!(dispatch(["start".into()].into_iter()).is_none());
        assert!(
            dispatch(["--blent-start-adb-server".into(), "extra".into()].into_iter())
                .unwrap()
                .is_err()
        );
    }

    #[test]
    fn t1465_missing_adb_does_not_disable_direct_usb_service() {
        let root = tempfile::tempdir().unwrap();
        assert!(start(true, Command::new(root.path().join("absent"))).is_ok());
    }

    fn fixture(root: &std::path::Path, status: i32) -> Command {
        let path = root.join("adb");
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$BLENT_TEST_ARGS\"\nexit {status}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut command = Command::new(path);
        command.env("BLENT_TEST_ARGS", root.join("args"));
        command
    }

    #[test]
    fn t1465_bootstrap_waits_for_original_client_and_reports_failure() {
        let root = tempfile::tempdir().unwrap();
        for status in [0, 1, 17] {
            assert_eq!(
                start(true, fixture(root.path(), status)).is_ok(),
                status == 0
            );
            assert_eq!(
                std::fs::read_to_string(root.path().join("args")).unwrap(),
                "-L\ntcp:5037\nstart-server\n"
            );
        }
        assert!(start(true, Command::new(root.path())).is_err());
    }
}
