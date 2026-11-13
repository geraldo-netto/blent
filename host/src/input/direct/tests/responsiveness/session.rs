use super::*;
use crate::session::{CaptureBackend, CaptureContext, CaptureWorkers, SessionResources, Spec};

struct Capture;
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        CaptureWorkers {
            capture: Box::pin(async move {
                let _ = context.stop.wait_for(|stop| *stop).await;
                Ok(())
            }),
            auxiliary: vec![],
        }
    }
}

#[tokio::test]
async fn t1402_session_retirement_retains_original_input_worker() {
    let (backend, _, gate, observe) = held();
    let _release = Release(gate.clone());
    let pending = backend
        .sink()
        .direct(Command::Negotiate { version: 1 })
        .unwrap();
    entered(observe).await;
    let original = backend.completion();
    let prepared = Spec {
        settings: crate::media::EncoderSettings {
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
        },
        instance: 0,
        ports: (0, 0),
        token: Some("t1402".into()),
        devices: (false, false, false),
    }
    .prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(Capture),
        Arc::new(backend),
    );
    let (_stop, stop) = watch::channel(false);
    let (runtime, listeners) = prepared.start_exported(stop).await.unwrap();
    let completion = runtime.stop().await;
    drop(pending);
    let retained = !completion.is_complete();
    gate.release();
    retired(original).await;
    retired(completion).await;
    drop(listeners);
    assert!(
        retained,
        "T1402 session retired before its original input worker"
    );
}
