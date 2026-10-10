//! T814: abort is not completion for a started native/blocking task.
use blent::{
    attachment::Attachment,
    media::EncoderSettings,
    session::{Runtime, Workers},
};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::sync::{oneshot, watch, Notify};
fn settings() -> EncoderSettings {
    EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: true,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}
#[tokio::test]
async fn t814_started_native_capture_cannot_hold_control_shutdown() {
    let (started, entered) = oneshot::channel();
    let (release, hold) = mpsc::channel();
    let admission = Workers::reserve().unwrap();
    let capture = tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        hold.recv().unwrap();
        Ok(())
    });
    entered.await.unwrap();
    let runtime = Runtime::from_workers(
        0,
        Attachment::new(watch::channel(settings()).0),
        Arc::new(Notify::new()),
        watch::channel(false).0,
        Workers::new(admission, capture, vec![]),
        (0, 0),
    );
    let stopped = tokio::time::timeout(Duration::from_secs(6), runtime.stop()).await;
    let pending = stopped.as_ref().is_ok_and(|ticket| !ticket.is_complete());
    release.send(()).unwrap();
    assert!(
        stopped.is_ok(),
        "T814: post-abort join blocked control shutdown"
    );
    assert!(
        pending,
        "T814: control return falsely confirmed native completion"
    );
    let ticket = stopped.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !ticket.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
