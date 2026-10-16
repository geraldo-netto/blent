//! T1028: real native owners outlive the session's parent task.
use super::*;
use blent::capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers};
use blent::input::backend::{InputBackend, InputSink, PenSample};
use blent_config::retirement::Completion;
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

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
        _: blent::input::InputConfig,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(std::future::pending())
    }
}
struct Held {
    armed: bool,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Drop for Held {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[derive(Clone, Copy)]
enum Resource {
    Helper,
    Stdout,
    StartupCanceled,
    StartupError,
    #[cfg(not(feature = "inproc-encoder"))]
    Encoder,
    #[cfg(not(feature = "inproc-encoder"))]
    Probe,
    #[cfg(not(feature = "inproc-encoder"))]
    Reader,
}
struct NativeCapture {
    config: CaptureConfig,
    resource: Resource,
    held: Held,
    ready: oneshot::Sender<Completion>,
}
impl NativeCapture {
    async fn expose(
        mut self,
        manager: &mut crate::capture::CaptureManager,
        context: &CaptureContext,
    ) -> Option<crate::capture::encoding::EncoderTask> {
        #[cfg(feature = "inproc-encoder")]
        let _ = context;
        self.held.armed = true;
        let (original, reader) = match self.resource {
            Resource::Helper | Resource::Stdout => {
                manager.helper.start(&self.config).await.unwrap();
                let original = match self.resource {
                    Resource::Helper => {
                        let child = manager.helper.child.as_mut().unwrap();
                        child.keep_alive(self.held);
                        child.completion()
                    }
                    _ => {
                        let task = manager.helper.stdout_task.as_mut().unwrap();
                        task.keep_alive(self.held);
                        task.completion()
                    }
                };
                (original, None)
            }
            Resource::StartupCanceled | Resource::StartupError => (
                Self::startup(&self.config, self.resource, self.held, manager).await,
                None,
            ),
            #[cfg(not(feature = "inproc-encoder"))]
            Resource::Probe => (Self::probe(&self.config, self.held, manager).await, None),
            #[cfg(not(feature = "inproc-encoder"))]
            Resource::Encoder | Resource::Reader => {
                Self::encoder(&self.config, self.resource, self.held, manager, context).await
            }
        };
        self.ready.send(original).unwrap();
        reader
    }
    async fn startup(
        config: &CaptureConfig,
        resource: Resource,
        held: Held,
        manager: &mut crate::capture::CaptureManager,
    ) -> Completion {
        let original = Arc::new(Mutex::new(None));
        let observed = original.clone();
        let mode = if matches!(resource, Resource::StartupCanceled) {
            "silent"
        } else {
            "bad"
        };
        let outcome = tokio::time::timeout(
            Duration::from_millis(100),
            manager.helper.start_with(config, |command| {
                command.env("BLENT_T1020_MODE", mode);
                let mut child = Child::spawn(command)?;
                *observed.lock().unwrap() = Some(child.completion());
                child.keep_alive(held);
                Ok(child)
            }),
        )
        .await;
        if matches!(resource, Resource::StartupCanceled) {
            assert!(outcome.is_err());
        } else {
            assert!(outcome.unwrap().is_err());
        }
        let retained = original.lock().unwrap().clone().unwrap();
        retained
    }
    #[cfg(not(feature = "inproc-encoder"))]
    async fn probe(
        config: &CaptureConfig,
        held: Held,
        manager: &crate::capture::CaptureManager,
    ) -> Completion {
        let original = Arc::new(Mutex::new(None));
        let observed = original.clone();
        let scope = blent::native_scope::Scope::new(manager.encoder.retirement.clone());
        let outcome = tokio::time::timeout(
            Duration::from_millis(100),
            crate::capture::probe::run_with(config, &scope, |_| {
                let mut command = Command::new("/bin/sleep");
                command
                    .arg("60")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped());
                let mut child = Child::spawn(&mut command)?;
                *observed.lock().unwrap() = Some(child.completion());
                child.keep_alive(held);
                Ok(child)
            }),
        )
        .await;
        assert!(
            outcome.is_err(),
            "T1032 fixture must cancel a live actual probe"
        );
        let proof = original.lock().unwrap().clone().unwrap();
        proof
    }
    #[cfg(not(feature = "inproc-encoder"))]
    async fn encoder(
        config: &CaptureConfig,
        resource: Resource,
        held: Held,
        manager: &mut crate::capture::CaptureManager,
        context: &CaptureContext,
    ) -> (Completion, Option<crate::capture::encoding::EncoderTask>) {
        manager
            .encoder
            .start_with(config, (640, 480), false, |_| {
                let mut command = Command::new("/bin/sleep");
                command.arg("60").stdout(Stdio::piped());
                Child::spawn(&mut command)
            })
            .unwrap();
        if matches!(resource, Resource::Encoder) {
            let child = manager.encoder.child.as_mut().unwrap();
            child.keep_alive(held);
            return (child.completion(), None);
        }
        let mut reader = manager
            .encoder
            .spawn_session(
                config,
                Some((640, 480)),
                crate::capture::encoding::EncoderOutput {
                    tx: context.video.clone(),
                    codec_config: manager.codec_config(),
                    latency: manager.latency_tracker(),
                    idr_wanted: manager.idr_request_flag(),
                },
            )
            .await
            .unwrap();
        reader.handle.keep_alive(held);
        (reader.handle.completion(), Some(reader))
    }
}
impl CaptureBackend for NativeCapture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        let mut manager = crate::capture::CaptureManager::new(self.config.clone());
        manager.bind_retirement(context.retirement.clone());
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                let mut reader = self.expose(&mut manager, &context).await;
                context.stop.wait_for(|stop| *stop).await.unwrap();
                if let Some(reader) = reader.as_mut() {
                    reader.finish(false).await.unwrap();
                }
                drop(reader);
                manager.shutdown().await;
                Ok(())
            }),
        }
    }
}
fn prepared(capture: NativeCapture) -> blent::session::Prepared {
    blent::session::Spec {
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
        token: None,
        devices: (false, false, false),
    }
    .prepare(
        watch::channel(false).0,
        blent::session::SessionResources::default(),
        Box::new(capture),
        Arc::new(Input),
    )
}
async fn completed(original: &Completion) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn exercise(resource: Resource) {
    let (entered_tx, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let (ready, started) = oneshot::channel();
    let capture = NativeCapture {
        config: startup_ownership_tests::configuration(),
        resource,
        held: Held {
            armed: false,
            entered: entered_tx,
            release: Mutex::new(held),
        },
        ready,
    };
    let (_daemon, stopped) = watch::channel(false);
    let (runtime, listeners) = prepared(capture).start_exported(stopped).await.unwrap();
    let original = started.await.unwrap();
    let completion = runtime.retirement.clone();
    runtime.stop().await;
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        !original.is_complete(),
        "T1028 fixture must hold actual native cleanup"
    );
    let retained = !completion.is_complete();
    drop(release);
    completed(&original).await;
    completed(&completion).await;
    drop(listeners);
    assert!(retained, "T1028 session released original native cleanup");
}
#[tokio::test]
async fn t1028_helper_native_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_helper_native_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::Helper).await;
}
#[tokio::test]
async fn t1028_stdout_native_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_stdout_native_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::Stdout).await;
}
#[cfg(not(feature = "inproc-encoder"))]
#[tokio::test]
async fn t1028_cli_child_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_cli_child_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::Encoder).await;
}
#[cfg(not(feature = "inproc-encoder"))]
#[tokio::test]
async fn t1028_reader_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_reader_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::Reader).await;
}

#[tokio::test]
async fn t1028_canceled_startup_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_canceled_startup_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::StartupCanceled).await;
}
#[tokio::test]
async fn t1028_failed_startup_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1028_failed_startup_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::StartupError).await;
}

#[cfg(not(feature = "inproc-encoder"))]
#[tokio::test]
async fn t1032_probe_cleanup_survives_whole_session_stop() {
    if crate::capture::ownership_tests::isolated("capture::helper::session_retirement_tests::t1032_probe_cleanup_survives_whole_session_stop") { return; }
    exercise(Resource::Probe).await;
}
