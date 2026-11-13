#![cfg(target_os = "linux")]
//! T1440: actual Wi-Fi CLI signal cancellation retains original native cleanup.
#[test]
fn t1440_wifi_cli_signals_retain_original_creation_and_cleanup() {
    let output = std::process::Command::new("/usr/bin/unshare")
        .args([
            "--user",
            "--map-root-user",
            "--mount",
            "--pid",
            "--fork",
            "--mount-proc",
            "--net",
            "/usr/bin/python3",
        ])
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wifi_cli/signals.py"))
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
