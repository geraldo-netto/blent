#![cfg(target_os = "linux")]
//! T1423: real daemon startup, isolated from devices, services and the ADB server.
#[test]
fn t1423_startup_signals_retain_cleanup_and_early_stop() {
    run_fixture("signals.py");
}

#[test]
fn t1428_failed_startup_removes_owned_pid() {
    run_fixture("pid_cleanup.py");
}

#[test]
fn t1509_actual_daemon_retains_pid_and_reports_pending_cleanup() {
    run_fixture("drain.py");
}

#[test]
fn t1695_concurrent_startup_has_one_owner_through_exit_drain() {
    run_fixture("startup_lock.py");
}

fn run_fixture(script: &str) {
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--pid",
            "--fork",
            "--mount-proc",
            "/usr/bin/python3",
        ])
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/daemon_startup")
                .join(script),
        )
        .arg(env!("CARGO_BIN_EXE_blent"))
        .output()
        .unwrap();
    print!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
