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
pub use crate::session_supervision::{SessionAdmission, Workers};

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
            activity: None,
            activity_preferences: Default::default(),
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
        }
    }
}

pub struct Prepared {
    pub activity_preferences: blent_config::host_activity::Settings,
    pub activity: Option<watch::Receiver<blent_config::host_activity::Publication>>,
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
}

impl Prepared {
    /// Bind an existing source view without creating another source or feed.
    pub fn with_activity(
        mut self,
        activity: Option<watch::Receiver<blent_config::host_activity::Publication>>,
    ) -> Self {
        self.activity = activity;
        self
    }
    pub fn with_activity_preferences(
        mut self,
        preferences: blent_config::host_activity::Settings,
    ) -> Self {
        self.activity_preferences = preferences;
        self
    }
    pub async fn start(self, daemon_stop: watch::Receiver<bool>) -> Result<Runtime> {
        self.start_exported(daemon_stop)
            .await
            .map(|(runtime, _listeners)| runtime)
    }

    /// Native composition seam for media registration. Portable session behavior
    /// still depends only on the capture/input and lifecycle contracts. The caller
    /// owns returned TCP capabilities independently until its route really retires.
    pub async fn start_exported(
        self,
        daemon_stop: watch::Receiver<bool>,
    ) -> Result<(Runtime, crate::session_network::RetainedListeners)> {
        let admission = Workers::reserve()?;
        self.start_exported_reserved(daemon_stop, admission).await
    }
    /// Composition retains admission.completion() before starting this future.
    /// All spawned work is installed in its retained owner before further startup.
    pub async fn start_exported_reserved(
        self,
        daemon_stop: watch::Receiver<bool>,
        admission: SessionAdmission,
    ) -> Result<(Runtime, crate::session_network::RetainedListeners)> {
        let (video, _) = video_queue::channel(
            video_queue::QUEUE_PACKETS,
            self.resources.idr_wanted.clone(),
        );
        let servers = BoundServers::bind(self.stream, self.input).await?;
        let (gate_tx, gate_rx) = watch::channel(false);
        let (stop_tx, stop_rx) = watch::channel(false);
        let mut retained_workers = Workers::empty(admission);
        let retirement = retained_workers.capture_lifetime();
        let workers = catch_unwind(AssertUnwindSafe(|| {
            self.capture.start(CaptureContext {
                activity: self.activity,
                activity_preferences: self.activity_preferences,
                retirement: retirement.clone(),
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
        let ports = servers.retained.ports();
        let retained = servers.start_retained(video, &mut retained_workers);
        retained_workers.add(
            "Display gate",
            spawn_display_gate(gate_tx, self.tablet.subscribe(), self.mode.subscribe()),
        );
        retained_workers.add(
            "Shutdown",
            forward_shutdown(daemon_stop, stop_rx, stop_tx.clone()),
        );
        for work in workers.auxiliary {
            retained_workers.add("Capture auxiliary", spawn_server("Capture auxiliary", work));
        }
        retained_workers.add(
            "Capture",
            spawn_server("Capture", retain_creation(workers.capture, retirement)),
        );
        Ok((
            Runtime::from_workers(
                self.instance,
                self.tablet,
                self.relaunch,
                stop_tx,
                retained_workers,
                ports,
            ),
            retained,
        ))
    }
}

fn retain_creation(
    work: crate::capture_contract::Work,
    retirement: Arc<dyn crate::capture_contract::NativeRetirement>,
) -> crate::capture_contract::Work {
    Box::pin(async move {
        let result = work.await;
        drop(retirement);
        result
    })
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
    pub retirement: blent_config::retirement::Completion,
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
        let retirement = workers.completion();
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
            retirement,
            supervisor,
            state,
        }
    }
    pub fn finished(&self) -> bool {
        self.supervisor.is_finished()
    }
    pub async fn stop(mut self) -> blent_config::retirement::Completion {
        self.tablet_tx.retire();
        self.stop_tx.send_replace(true);
        let _ = (&mut self.supervisor).await;
        self.retirement.clone()
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
    let (stream, input, _retained) = BoundServers::bind(stream, input).await?.start(video);
    Ok((stream, input))
}

struct BoundServers {
    stream: stream::StreamServer,
    input: input::InputServer,
    video_listener: tokio::net::TcpListener,
    input_listener: tokio::net::TcpListener,
    retained: crate::session_network::RetainedListeners,
}
impl BoundServers {
    async fn bind(stream: stream::StreamServer, input: input::InputServer) -> Result<Self> {
        let video_listener = stream.bind().await?;
        let input_listener = input.bind().await?;
        let (retained, video_listener, input_listener) =
            crate::session_network::RetainedListeners::retain(video_listener, input_listener)?;
        Ok(Self {
            stream,
            input,
            video_listener,
            input_listener,
            retained,
        })
    }
    fn start(
        self,
        video: video_queue::VideoSender,
    ) -> (
        JoinHandle<Result<()>>,
        JoinHandle<Result<()>>,
        crate::session_network::RetainedListeners,
    ) {
        let (stream, input, retained) = self.work(video);
        (
            spawn_server("Stream", stream),
            spawn_server("Input", input),
            retained,
        )
    }
    fn start_retained(
        self,
        video: video_queue::VideoSender,
        workers: &mut Workers,
    ) -> crate::session_network::RetainedListeners {
        let (stream, input, retained) = self.work(video);
        workers.add("Stream", spawn_server("Stream", stream));
        workers.add("Input", spawn_server("Input", input));
        retained
    }
    /// Both ownership adapters share pure listener-work construction. Spawning
    /// remains separate so retained startup records each exposed worker promptly.
    fn work(
        self,
        video: video_queue::VideoSender,
    ) -> (
        impl std::future::Future<Output = Result<()>> + Send + 'static,
        impl std::future::Future<Output = Result<()>> + Send + 'static,
        crate::session_network::RetainedListeners,
    ) {
        let stream = async move {
            self.stream
                .run_with_listener(video, self.video_listener)
                .await
        };
        let input = async move { self.input.run_with_listener(self.input_listener).await };
        (stream, input, self.retained)
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
