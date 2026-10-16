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

#[path = "native_retirement_tests.rs"]
mod native_retirement_tests;
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
fn prepared(capture: Box<dyn CaptureBackend>, ports: (u16, u16)) -> crate::session::Prepared {
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
    Spec {
        settings,
        instance: 0,
        ports,
        token: None,
        devices: (false, false, false),
    }
    .prepare(
        watch::channel(false).0,
        SessionResources::default(),
        capture,
        Arc::new(Input),
    )
}
async fn runtime() -> (
    Runtime,
    crate::session_network::RetainedListeners,
    Arc<AtomicBool>,
    watch::Sender<bool>,
) {
    let done = Arc::new(AtomicBool::new(false));
    let prepared = prepared(Box::new(Capture(done.clone())), (0, 0));
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

struct PanickingCapture;
impl CaptureBackend for PanickingCapture {
    fn start(self: Box<Self>, _: CaptureContext) -> CaptureWorkers {
        panic!("T928 capture constructor fails before worker exposure");
    }
}
#[tokio::test]
async fn t928_capture_startup_panic_drops_partial_listeners_before_completion_reuse() {
    let video = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let input = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ports = (
        video.local_addr().unwrap().port(),
        input.local_addr().unwrap().port(),
    );
    drop((video, input));
    let owner = crate::session::Workers::reserve().unwrap();
    let completion = owner.completion();
    assert!(!completion.is_complete());
    let (_stop, stopped) = watch::channel(false);
    let result = prepared(Box::new(PanickingCapture), ports)
        .start_exported_reserved(stopped, owner)
        .await;
    assert!(matches!(result, Err(error) if error.to_string()=="Capture startup panicked"));
    assert!(completion.is_complete());
    for port in [ports.0, ports.1] {
        std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    }
}

// T928: retained startup must preserve the legacy served-listener contract too.
#[tokio::test]
async fn t928_legacy_server_work_serves_both_channels_and_releases_owned_ports() {
    use futures_util::StreamExt;
    use tokio::io::AsyncReadExt;
    let video = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let input = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ports = (
        video.local_addr().unwrap().port(),
        input.local_addr().unwrap().port(),
    );
    drop((video, input));
    let prepared = prepared(Box::new(PanickingCapture), ports);
    prepared
        .resources
        .codec_config
        .publish(Some(crate::media_storage::MediaBytes::from_static(b"csd")));
    let (video, _) = crate::video_queue::channel(8, prepared.resources.idr_wanted);
    let (stream, input) = crate::session::start_servers(prepared.stream, prepared.input, video)
        .await
        .unwrap();
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", ports.0))
        .await
        .unwrap();
    let mut config = [0; 8];
    tokio::time::timeout(Duration::from_secs(3), client.read_exact(&mut config))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&config[5..], b"csd");
    let (mut control, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}", ports.1))
        .await
        .unwrap();
    let greeting = tokio::time::timeout(Duration::from_secs(3), control.next())
        .await
        .unwrap();
    assert!(matches!(
        greeting,
        Some(Ok(tokio_tungstenite::tungstenite::Message::Text(_)))
    ));
    drop((client, control));
    stream.abort();
    input.abort();
    assert!(stream.await.unwrap_err().is_cancelled());
    assert!(input.await.unwrap_err().is_cancelled());
    for port in [ports.0, ports.1] {
        std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    }
}
