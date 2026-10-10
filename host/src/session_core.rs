//! Portable session ownership. Native capture and input arrive as explicit adapters.
use crate::{attachment::Attachment, input, latency::LatencyTracker, media, stream, video_queue};
use anyhow::Result;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;
use tracing::error;

pub use crate::capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers};
pub use crate::session_supervision::Workers;

/// Composition-owned bindings for the current stream/input implementations.
/// These are deliberately outside the capture backend contract.
#[derive(Clone, Default)]
pub struct SessionResources {
    pub codec_config: media::CodecConfig,
    pub idr_wanted: Arc<AtomicBool>,
    pub latency: LatencyTracker,
}

pub struct Spec {
    pub settings: media::EncoderSettings,
    pub instance: u32,
    pub ports: (u16, u16),
    pub token: Option<String>,
    pub devices: (bool, bool, bool),
}

impl Spec {
    fn input_config(&self) -> input::InputConfig {
        input::InputConfig {
            port: self.ports.1,
            instance: self.instance,
            token: self.token.clone(),
            codec: media::Codec::from_encoder(&self.settings.encoder)
                .wire_name()
                .into(),
            virtual_width: self.settings.width,
            virtual_height: self.settings.height,
            touch: self.devices.0,
            pen: self.devices.1,
            pointer: self.devices.2,
        }
    }

    /// Prepare state without opening sockets or starting native workers.
    pub fn prepare(
        self,
        mode: watch::Sender<bool>,
        resources: SessionResources,
        capture: Box<dyn CaptureBackend>,
        backend: Arc<dyn input::backend::InputBackend>,
    ) -> Prepared {
        let config = self.input_config();
        let (settings, settings_rx) = watch::channel(self.settings);
        let tablet = Attachment::with_token(settings.clone(), self.token.clone());
        let relaunch = Arc::new(Notify::new());
        let stream = stream::StreamServer::new(
            stream::StreamConfig {
                video_port: self.ports.0,
                token: self.token,
            },
            resources.codec_config.clone(),
            resources.idr_wanted.clone(),
        )
        .with_attachment(tablet.clone());
        let input = input::InputServer::with_backend(
            config,
            Some(settings.clone()),
            mode.clone(),
            resources.latency.clone(),
            relaunch.clone(),
            tablet.subscribe(),
            backend,
        )
        .with_attachment(tablet.clone());
        Prepared {
            settings,
            settings_rx,
            tablet,
            relaunch,
            mode,
            capture,
            resources,
            stream,
            input,
            instance: self.instance,
            ports: self.ports,
        }
    }
}

pub struct Prepared {
    pub settings: watch::Sender<media::EncoderSettings>,
    pub settings_rx: watch::Receiver<media::EncoderSettings>,
    pub tablet: Attachment,
    pub relaunch: Arc<Notify>,
    pub resources: SessionResources,
    pub stream: stream::StreamServer,
    pub input: input::InputServer,
    mode: watch::Sender<bool>,
    capture: Box<dyn CaptureBackend>,
    instance: u32,
    ports: (u16, u16),
}

impl Prepared {
    pub async fn start(self, daemon_stop: watch::Receiver<bool>) -> Result<Runtime> {
        let (video, _) = video_queue::channel(
            video_queue::QUEUE_PACKETS,
            self.resources.idr_wanted.clone(),
        );
        let servers = BoundServers::bind(self.stream, self.input).await?;
        let (gate_tx, gate_rx) = watch::channel(false);
        let (stop_tx, stop_rx) = watch::channel(false);
        let workers = catch_unwind(AssertUnwindSafe(|| {
            self.capture.start(CaptureContext {
                video: video.clone().into(),
                headers: Arc::new(self.resources.codec_config),
                refresh: self.resources.idr_wanted,
                settings: self.settings,
                settings_rx: self.settings_rx,
                display: gate_rx,
                stop: stop_rx.clone(),
                identity: Arc::new(self.tablet.clone()),
            })
        }))
        .map_err(|_| anyhow::anyhow!("Capture startup panicked"))?;
        let (stream, input) = servers.start(video);
        let gate = spawn_display_gate(gate_tx, self.tablet.subscribe(), self.mode.subscribe());
        let stop = forward_shutdown(daemon_stop, stop_rx, stop_tx.clone());
        let mut tasks = vec![
            ("Stream", stream),
            ("Input", input),
            ("Display gate", gate),
            ("Shutdown", stop),
        ];
        tasks.extend(
            workers
                .auxiliary
                .into_iter()
                .map(|work| ("Capture auxiliary", spawn_server("Capture auxiliary", work))),
        );
        Ok(Runtime::from_workers(
            self.instance,
            self.tablet,
            self.relaunch,
            stop_tx,
            Workers::new(spawn_server("Capture", workers.capture), tasks),
            self.ports,
        ))
    }
}

/// The supervisor owns task handles; consumers observe independent state values.
pub struct Runtime {
    pub instance: u32,
    pub tablet_tx: Attachment,
    pub relaunch: Arc<Notify>,
    pub stop_tx: watch::Sender<bool>,
    pub video_port: u16,
    pub input_port: u16,
    pub status: watch::Receiver<crate::session_state::State>,
    supervisor: JoinHandle<()>,
    state: watch::Sender<crate::session_state::State>,
}
impl Runtime {
    pub fn from_workers(
        instance: u32,
        tablet_tx: Attachment,
        relaunch: Arc<Notify>,
        stop_tx: watch::Sender<bool>,
        workers: Workers,
        ports: (u16, u16),
    ) -> Self {
        let (state, status) = watch::channel(crate::session_state::State::Running);
        let supervisor = tokio::spawn(crate::session_supervision::run(
            workers,
            tablet_tx.clone(),
            stop_tx.clone(),
            state.clone(),
        ));
        Self {
            instance,
            tablet_tx,
            relaunch,
            stop_tx,
            video_port: ports.0,
            input_port: ports.1,
            status,
            supervisor,
            state,
        }
    }
    pub async fn stop(mut self) {
        self.tablet_tx.retire();
        self.stop_tx.send_replace(true);
        let _ = (&mut self.supervisor).await;
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.tablet_tx.retire();
        self.stop_tx.send_replace(true);
        self.supervisor.abort();
        self.state.send_if_modified(|state| {
            if matches!(state, crate::session_state::State::Running) {
                *state = crate::session_state::State::Cancelled;
                true
            } else {
                false
            }
        });
    }
}

pub fn forward_shutdown(
    mut daemon: watch::Receiver<bool>,
    mut local: watch::Receiver<bool>,
    capture: watch::Sender<bool>,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        tokio::select! {
            _ = daemon.wait_for(|stop| *stop) => {},
            _ = local.wait_for(|stop| *stop) => {},
        }
        let _ = capture.send(true);
        Ok(())
    })
}

pub async fn start_servers(
    stream: stream::StreamServer,
    input: input::InputServer,
    video: crate::video_queue::VideoSender,
) -> Result<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)> {
    Ok(BoundServers::bind(stream, input).await?.start(video))
}

struct BoundServers {
    stream: stream::StreamServer,
    input: input::InputServer,
    video_listener: tokio::net::TcpListener,
    input_listener: tokio::net::TcpListener,
}
impl BoundServers {
    async fn bind(stream: stream::StreamServer, input: input::InputServer) -> Result<Self> {
        let video_listener = stream.bind().await?;
        let input_listener = input.bind().await?;
        Ok(Self {
            stream,
            input,
            video_listener,
            input_listener,
        })
    }
    fn start(
        self,
        video: video_queue::VideoSender,
    ) -> (JoinHandle<Result<()>>, JoinHandle<Result<()>>) {
        let stream = spawn_server("Stream", async move {
            self.stream
                .run_with_listener(video, self.video_listener)
                .await
        });
        let input = spawn_server("Input", async move {
            self.input.run_with_listener(self.input_listener).await
        });
        (stream, input)
    }
}

pub fn spawn_server(
    name: &'static str,
    run: impl std::future::Future<Output = Result<()>> + Send + 'static,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let outcome = run.await;
        if let Err(error) = &outcome {
            error!("{name} server failed: {error}");
        }
        outcome
    })
}

pub fn spawn_display_gate(
    gate: watch::Sender<bool>,
    mut tablet: watch::Receiver<bool>,
    mut mode: watch::Receiver<bool>,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let mut last = false;
        loop {
            let attached = *tablet.borrow();
            let active = attached && !*mode.borrow();
            if active != last {
                last = active;
                let _ = gate.send(active);
            }
            tokio::select! {
                r = tablet.changed() => if r.is_err() { break },
                r = mode.changed() => if r.is_err() { break },
            }
        }
        Ok(())
    })
}
