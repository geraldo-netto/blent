#![cfg(target_os = "linux")]
//! T1435: actual main-thread GUI startup/close on an owned display; no nested Cargo.
#[test]
fn t1435_actual_gui_entry_closes_on_private_display() {
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--pid",
            "--fork",
            "--mount-proc",
            "--net",
            "/bin/sh",
            "-c",
            "\"$@\" & child=$!; wait \"$child\"",
            "sh",
            "/usr/bin/xvfb-run",
            "-a",
            "/usr/bin/dbus-run-session",
            "--",
            "/usr/bin/python3",
        ])
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/process_exit.py"))
        .arg(env!("CARGO_BIN_EXE_blent-gui"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
