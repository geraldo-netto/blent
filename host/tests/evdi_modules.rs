#![cfg(target_os = "linux")]
//! Native C contract and capture-adapter regressions without physical devices.
use std::{path::PathBuf, process::Command};

fn run_capture_fixture(source: &str, extra: &[&str]) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("capture-fixture");
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
        .args(extra)
        .arg(root.join("tests").join(source))
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
    let log = String::from_utf8_lossy(&tested.stderr).into_owned();
    assert!(tested.status.success(), "{log}");
    log
}

#[test]
fn t1712_quiet_capture_descriptors_avoid_redundant_probes() {
    eprint!("{}", run_capture_fixture("capture_probe_test.c", &[]));
}

#[test]
fn t1713_fresh_frames_skip_advice_but_idle_keeps_exact_deadlines() {
    eprint!(
        "{}",
        run_capture_fixture(
            "writer_advice_probe_test.c",
            &["-Wl,--wrap=pthread_cond_timedwait"]
        )
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
