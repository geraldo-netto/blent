#![cfg(target_os = "linux")]
//! T1021: cross-language exec and native retained-monitor pause integration.
use blent_config::commands::OwnedChild;
use blent_config::linux::capture_control::Control;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

fn worker(directory: &std::path::Path) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary = directory.join("capture-pause");
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
        .arg(root.join("tests/capture_pause_worker.c"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    binary
}

#[tokio::test]
async fn t1021_rust_controls_original_native_monitor_across_repeated_pauses() {
    let directory = tempfile::tempdir().unwrap();
    let binary = worker(directory.path());
    let (mut control, inherited) = Control::pair().unwrap();
    let mut command = tokio::process::Command::new(binary);
    command.stdout(Stdio::null());
    inherited.attach(&mut command).unwrap();
    let mut child = OwnedChild::spawn(&mut command).unwrap();
    drop(command);
    child.keep_alive(control.lease());
    let mut sequence = 0;
    for _ in 0..8 {
        for active in [true, false] {
            let proof = tokio::time::timeout(Duration::from_secs(3), control.set_active(active))
                .await
                .unwrap()
                .unwrap();
            sequence += 1;
            assert_eq!(proof.sequence(), sequence);
            assert_eq!(proof.active(), active);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    unsafe { libc::kill(child.id().unwrap() as i32, libc::SIGTERM) };
    assert!(child
        .finish(Duration::from_secs(3))
        .await
        .unwrap()
        .success());
    assert!(control.set_active(true).await.is_err());
}
