use super::*;
use crate::{
    capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers},
    input::{
        backend::{InputBackend, InputSink, PenSample},
        InputConfig,
    },
    media::EncoderSettings,
    session::{SessionResources, Spec},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
struct Input;
impl InputSink for Input {
    fn release_all(&self) {}
    fn touch(&self, _: (f64, f64, f64), _: u8, _: u8) {}
    fn pen(&self, _: PenSample, _: bool) {}
}
impl InputBackend for Input {
    fn sink(&self) -> Arc<dyn InputSink> {
        Arc::new(Self)
    }
    fn follow(
        &self,
        _: watch::Receiver<bool>,
        _: watch::Receiver<bool>,
        _: InputConfig,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(std::future::pending())
    }
}
struct Capture(Arc<AtomicBool>);
struct Finished(Arc<AtomicBool>);
impl Drop for Finished {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        let finished = Finished(self.0);
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                let _finished = finished;
                context.stop.wait_for(|stop| *stop).await.unwrap();
                Ok(())
            }),
        }
    }
}
async fn runtime() -> (
    Runtime,
    crate::session_network::RetainedListeners,
    Arc<AtomicBool>,
    watch::Sender<bool>,
) {
    let done = Arc::new(AtomicBool::new(false));
    let settings = EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    };
    let prepared = Spec {
        settings,
        instance: 0,
        ports: (0, 0),
        token: None,
        devices: (false, false, false),
    }
    .prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(Capture(done.clone())),
        Arc::new(Input),
    );
    let (daemon, stop) = watch::channel(false);
    let (runtime, listeners) = prepared.start_exported(stop).await.unwrap();
    (runtime, listeners, done, daemon)
}
async fn retired(work: &RuntimeWork) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !work.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t925_owned_runtime_waits_for_supervisor_and_native_completion() {
    let (runtime, listeners, done, _daemon) = runtime().await;
    let ports = listeners.ports();
    let mut work = RuntimeWork::owned(runtime);
    assert!(work.alive());
    assert!(!work.retired());
    tokio::task::yield_now().await;
    work.close();
    work.close();
    assert!(!work.alive());
    assert!(!work.retired());
    retired(&work).await;
    assert!(done.load(Ordering::SeqCst));
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
    }
    drop(work);
    drop(listeners);
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}
#[tokio::test]
async fn t925_shared_primary_close_preserves_daemon_owned_capture() {
    let (runtime, listeners, done, _daemon) = runtime().await;
    let mut work = RuntimeWork::shared(&runtime);
    assert!(work.alive());
    assert!(!work.retired());
    work.close();
    work.close();
    assert!(work.retired());
    assert!(!work.alive());
    assert!(!*runtime.stop_tx.borrow());
    assert!(!runtime.finished());
    assert!(!done.load(Ordering::SeqCst));
    drop(work);
    let mut observer = RuntimeWork::shared(&runtime);
    assert!(observer.alive());
    tokio::task::yield_now().await;
    runtime.stop_tx.send_replace(true);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !runtime.finished() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(!observer.alive());
    observer.close();
    assert!(observer.retired());
    assert!(done.load(Ordering::SeqCst));
    drop(runtime);
    drop(listeners);
}
#[tokio::test]
async fn t925_dropped_owned_runtime_requests_cleanup_without_waiting() {
    let (runtime, listeners, done, _daemon) = runtime().await;
    let completion = runtime.retirement.clone();
    tokio::task::yield_now().await;
    drop(RuntimeWork::owned(runtime));
    tokio::time::timeout(Duration::from_secs(3), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(done.load(Ordering::SeqCst));
    drop(listeners);
}
