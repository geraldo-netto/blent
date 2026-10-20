//! T1126: one original decoder admission, before subprocess creation.
use super::*;
use crate::camera::{
    protocol,
    tests::{new_report, options as new_options, script},
};
use crate::camera_control::NativeResource;
use futures_util::FutureExt;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn t1126_pending_original_decoder_rejects_native_replacement_before_spawn() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("created");
    let ffmpeg = script(
        root.path(),
        "decoder",
        &format!("open({marker:?}, 'w').close()"),
    );
    let report = new_report();
    let reserved = report.reserve_native(NativeResource::Decoder).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let token = "ab".repeat(32);
    client
        .write_all(&[protocol::MAGIC.as_slice(), token.as_bytes(), &[0, 0]].concat())
        .await
        .unwrap();
    client.shutdown().await.unwrap();
    let frames = [
        tokio::sync::watch::channel(None).0,
        tokio::sync::watch::channel(None).0,
    ];
    let result = connection(
        socket,
        &token,
        &ffmpeg,
        &new_options().profile,
        &frames,
        report.clone(),
    )
    .await;
    drop(reserved);
    report.wait_native().await;
    assert!(result.is_err());
    assert!(
        !marker.exists(),
        "T1126 replacement decoder started before original retirement"
    );
}

#[tokio::test]
async fn t1126_live_decoder_is_observed_until_its_cancelled_future_retires() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("created");
    let ffmpeg = script(
        root.path(),
        "decoder",
        &format!("import sys\nopen({marker:?}, 'w').close()\nsys.stdin.buffer.read()"),
    );
    let report = new_report();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let token = "ab".repeat(32);
    client
        .write_all(&[protocol::MAGIC.as_slice(), token.as_bytes(), &[0, 0]].concat())
        .await
        .unwrap();
    let frames = [
        tokio::sync::watch::channel(None).0,
        tokio::sync::watch::channel(None).0,
    ];
    let options = new_options();
    let mut work = Box::pin(connection(
        socket,
        &token,
        &ffmpeg,
        &options.profile,
        &frames,
        report.clone(),
    ));
    let ready = tokio::select! {
        _ = &mut work => false,
        result = tokio::time::timeout(std::time::Duration::from_secs(5), created(&marker)) => result.is_ok(),
    };
    let retained = report.wait_native().now_or_never().is_none();
    drop(work);
    tokio::time::timeout(std::time::Duration::from_secs(3), report.wait_native())
        .await
        .unwrap();
    assert!(
        ready && retained,
        "T1126 live decoder original proof was not retained"
    );
}
async fn created(path: &std::path::Path) {
    loop {
        if path.exists() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
