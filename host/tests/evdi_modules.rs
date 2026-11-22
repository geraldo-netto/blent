#![cfg(target_os = "linux")]
//! Native C contract and capture-adapter regressions without physical devices.
use std::{path::PathBuf, process::Command};

#[test]
fn t1712_quiet_capture_descriptors_avoid_redundant_probes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("probe-test");
    let compiled = Command::new("cc")
        .args([
            "-std=c11",
            "-O1",
            "-g",
            "-pthread",
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
            "-fsanitize=address,undefined",
            "-fno-pie",
            "-no-pie",
        ])
        .arg(root.join("tests/capture_probe_test.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let tested = Command::new(binary).output().unwrap();
    eprint!("{}", String::from_utf8_lossy(&tested.stderr));
    assert!(
        tested.status.success(),
        "{}",
        String::from_utf8_lossy(&tested.stderr)
    );
}

#[test]
fn t1709_cadence_policy_needs_no_native_adapter() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("cadence-test");
    let compiled = Command::new("cc")
        .args([
            "-std=c11",
            "-O1",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=address,undefined",
            "-fno-pie",
            "-no-pie",
            "-I",
        ])
        .arg(root.join("evdi"))
        .arg(root.join("tests/capture_cadence_policy.c"))
        .arg(root.join("evdi/capture_cadence.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let tested = Command::new(binary).output().unwrap();
    assert!(
        tested.status.success(),
        "{}",
        String::from_utf8_lossy(&tested.stderr)
    );
}

#[test]
fn t380_independent_contexts_preserve_frame_and_callback_lifetimes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("modules-test");
    let output = Command::new("cc")
        .args([
            "-std=c11",
            "-O1",
            "-g",
            "-pthread",
            "-fsanitize=address,undefined",
            "-fno-pie",
            "-no-pie",
            "-I",
        ])
        .arg(root.join("evdi"))
        .arg(root.join("tests/evdi_modules_test.c"))
        .args(
            [
                "conversion.c",
                "frame_exchange.c",
                "fifo_writer.c",
                "capture.c",
                "capture_cadence.c",
                "capture_activity.c",
                "capture_control.c",
                "cursor_overlay.c",
                "raw_ring.c",
                "writer.c",
                "idle_control.c",
            ]
            .map(|name| root.join("evdi").join(name)),
        )
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
