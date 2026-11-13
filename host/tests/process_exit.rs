#![cfg(target_os = "linux")]
//! T1411: admitted session/routes settle before the process runtime disappears.
use blent_config::commands::SyncCommandExt;
use std::{process::Command, time::Duration};
#[path = "process_exit/fixture.rs"]
mod fixture;

#[test]
fn t1411_process_exit_fixture() {
    if let Ok(mode) = std::env::var("BLENT_T1411_EXIT") {
        fixture::run(&mode);
    }
}

#[test]
fn t1411_process_exit_retains_routes_and_async_native_session_cleanup() {
    for mode in ["normal", "error", "cancel", "panic"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "t1411_process_exit_fixture", "--nocapture"])
            .env("BLENT_T1411_EXIT", mode)
            .env("BLENT_RUNTIME_STOP_MS", "20")
            .env("BLENT_SESSION_STOP_MS", "20")
            .output_timeout(Duration::from_secs(15))
            .unwrap();
        assert!(
            output.status.success(),
            "T1411 {mode}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
