#![cfg(target_os = "linux")]
#[test]
fn t765_guest_probe_handles_native_failures_without_hardware() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("probe-test");
    let build = std::process::Command::new("cc").args([
        "-std=c11", "-O1", "-g", "-fsanitize=address,undefined", "-fno-sanitize-recover=all",
        "-fno-pie", "-no-pie", "-I/usr/include/libdrm", "-I",
    ]).arg(root.join("evdi")).arg(root.join("tests/kms_probe.c"))
        .arg("-o").arg(&binary).output().unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let output = std::process::Command::new(binary).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
