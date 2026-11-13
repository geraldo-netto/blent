#![cfg(target_os = "linux")]
//! T1423: real daemon startup, isolated from devices, services and the ADB server.
#[test]
fn t1423_startup_signals_retain_cleanup_and_early_stop() {
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
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/daemon_startup/signals.py"
        ))
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
