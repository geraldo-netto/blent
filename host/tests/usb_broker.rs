//! T853: exercise the actual broker executable without touching USB devices.
#![cfg(all(target_os = "linux", feature = "accessory-transport"))]
use blent::broker::linux::{async_io::AsyncSocket, Socket};
use blent_config::{
    commands::OwnedChild,
    connection::{
        control::{Command, Notice},
        registration::Message,
    },
};
use std::{
    future::Future, os::unix::fs::PermissionsExt, path::Path, process::Stdio, time::Duration,
};

async fn bounded<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), work)
        .await
        .unwrap()
}
fn executable() -> tokio::process::Command {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_blent-usb-broker"));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
fn start(root: &Path) -> OwnedChild {
    let mut command = executable();
    command
        .arg("--runtime")
        .arg(root.join("runtime"))
        .arg("--state")
        .arg(root.join("state"))
        .arg("--helper")
        .arg(root.join("helper"));
    OwnedChild::spawn(&mut command).unwrap()
}
async fn connect(root: &Path) -> AsyncSocket {
    bounded(async {
        loop {
            if let Ok(socket) = Socket::connect(&root.join("runtime/broker.sock"), unsafe {
                libc::geteuid()
            }) {
                return AsyncSocket::new(socket).unwrap();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
}
async fn identity(socket: &AsyncSocket) -> String {
    bounded(async {
        loop {
            socket.send(&Message::Control { request: 1, command: Command::Inspect }, &[]).await.unwrap();
            match socket.receive().await.unwrap().message {
                Message::Rejected { .. } => tokio::time::sleep(Duration::from_millis(10)).await,
                Message::Report { notice: Notice::Begin { revision, local }, .. } => {
                    assert!(matches!(socket.receive().await.unwrap().message,
                        Message::Report { notice: Notice::End { revision: end }, .. } if end == revision));
                    return local.as_str().to_owned();
                }
                _ => panic!("unexpected inventory response"),
            }
        }
    }).await
}
async fn stop(child: OwnedChild) {
    assert_eq!(
        unsafe { libc::kill(child.id().unwrap() as i32, libc::SIGTERM) },
        0
    );
    assert!(child
        .finish(Duration::from_secs(8))
        .await
        .unwrap()
        .success());
}

#[tokio::test]
async fn t853_executable_election_client_death_signal_and_identity_restart() {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("helper");
    std::fs::write(&helper, b"#!/bin/sh\n[ \"$1\" = --list ] || exit 7\nprintf 'BLUI\\000\\000\\000\\001\\000\\000\\000\\000\\000\\000\\000\\000'\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let child = start(root.path());
    let first = connect(root.path()).await;
    let fingerprint = identity(&first).await;
    assert!(!start(root.path())
        .finish(Duration::from_secs(5))
        .await
        .unwrap()
        .success());
    let second = connect(root.path()).await;
    drop(first);
    assert_eq!(identity(&second).await, fingerprint);
    stop(child).await;
    assert!(!root.path().join("runtime/broker.sock").exists());
    assert!(bounded(second.receive()).await.is_err());
    let restarted = start(root.path());
    assert_eq!(identity(&connect(root.path()).await).await, fingerprint);
    stop(restarted).await;
    assert!(!root.path().join("runtime/broker.sock").exists());
}

#[tokio::test]
async fn t853_executable_help_and_invalid_configuration_exit_boundedly() {
    let mut help = executable();
    help.arg("--help");
    assert!(OwnedChild::spawn(&mut help)
        .unwrap()
        .finish(Duration::from_secs(5))
        .await
        .unwrap()
        .success());
    let mut invalid = executable();
    invalid.args(["--links", "0"]);
    assert!(!OwnedChild::spawn(&mut invalid)
        .unwrap()
        .finish(Duration::from_secs(5))
        .await
        .unwrap()
        .success());
}
