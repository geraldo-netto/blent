#![cfg(target_os = "linux")]
//! T1421: original source/native ownership survives process-entry teardown.
use blent_config::commands::SyncCommandExt;
use std::{process::Command, time::Duration};
#[path = "activity_exit/fixture.rs"]
mod fixture;

#[test]
fn t1421_activity_exit_fixture() {
    if let Ok(mode) = std::env::var("BLENT_T1421_EXIT") {
        fixture::run(&mode);
    }
}

#[test]
fn t1421_exit_retains_delayed_activity_creation_and_cleanup() {
    for mode in ["delayed", "normal", "error", "cancel", "panic"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "t1421_activity_exit_fixture", "--nocapture"])
            .env("BLENT_T1421_EXIT", mode)
            .env("BLENT_RUNTIME_STOP_MS", "20")
            .output_timeout(Duration::from_secs(20))
            .unwrap();
        assert!(
            output.status.success(),
            "T1421 {mode}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
