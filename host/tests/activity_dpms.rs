//! T1013: execute the real read-only helper; never change the desktop's power.
#![cfg(target_os = "linux")]
use blent_config::{commands::OwnedChild, host_activity::DisplayObservation};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpListener, process::Command};

#[test]
fn t1013_real_helper_dispatches_before_daemon_startup_on_private_x11() {
    let output = std::process::Command::new("xvfb-run")
        .args([
            "-a",
            "-s",
            "-screen 0 64x64x24 -nolisten tcp -extension DPMS",
        ])
        .arg(env!("CARGO_BIN_EXE_blent"))
        .args(["--blent-activity-dpms", "1", "1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        DisplayObservation::decode(&output.stdout).unwrap(),
        DisplayObservation {
            supported: false,
            on: None
        }
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blent"))
        .args(["--blent-activity-dpms", "0", "1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blent"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}

#[tokio::test]
async fn t1013_real_blocked_xlib_connection_keeps_original_process_until_retirement() {
    let server = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = server.local_addr().unwrap().port();
    assert!(port > 6000);
    let mut command = Command::new(env!("CARGO_BIN_EXE_blent"));
    command
        .args(["--blent-activity-dpms", "1", "1"])
        .env("DISPLAY", format!("127.0.0.1:{}", port - 6000))
        .env("XAUTHORITY", "/__blent_missing_xauthority__")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = OwnedChild::spawn(&mut command).unwrap();
    let proof = child.completion();
    let (mut peer, _) = tokio::time::timeout(Duration::from_secs(3), server.accept())
        .await
        .unwrap()
        .unwrap();
    let mut setup = [0; 12];
    tokio::time::timeout(Duration::from_secs(3), peer.read_exact(&mut setup))
        .await
        .unwrap()
        .unwrap();
    let mut output = child.take_stdout().unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), output.read_u8())
            .await
            .is_err()
    );
    assert!(!proof.is_complete());
    child.retire(Duration::ZERO).await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(output.read_u8().await.is_err());
}
