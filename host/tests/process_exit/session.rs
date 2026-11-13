use super::State;
use blent::{
    capture_contract::*,
    input::{
        backend::{InputBackend, InputSink, PenSample},
        InputConfig,
    },
    media::EncoderSettings,
    session::{Runtime, SessionResources, Spec},
};
use blent_config::retirement::Pool;
use std::sync::{atomic::Ordering, Arc};
use tokio::sync::watch;
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
    ) -> Signal<'static> {
        Box::pin(std::future::pending())
    }
}
struct Capture(Arc<State>);
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, context: CaptureContext) -> CaptureWorkers {
        let native = Pool::new(1).unwrap().reserve().unwrap();
        context.retirement.retain(native.completion());
        tokio::spawn(async move {
            self.0.wake.notified().await;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            self.0.native_done.store(true, Ordering::Release);
            drop(native);
        });
        CaptureWorkers {
            capture: Box::pin(std::future::pending()),
            auxiliary: vec![],
        }
    }
}
pub(super) async fn create(state: Arc<State>) -> (Runtime, watch::Sender<bool>) {
    let spec = Spec {
        instance: 0,
        ports: (0, 0),
        token: None,
        devices: (false, false, false),
        settings: settings(),
    };
    let (shutdown, stopped) = watch::channel(false);
    let runtime = spec
        .prepare(
            watch::channel(false).0,
            SessionResources::default(),
            Box::new(Capture(state)),
            Arc::new(Input),
        )
        .start(stopped)
        .await
        .unwrap();
    (runtime, shutdown)
}
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
