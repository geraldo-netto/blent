//! T1016 real helper/encoder/packet generation behind joined USB registration.
use super::*;
use blent::{
    capture_contract::{CaptureBackend, CaptureContext, CaptureWorkers, VideoOutput},
    connection_discovery::Discovery,
    display_connections::{Connections, Decision},
    display_slots::Slots,
    registered_session::{ActivityBinding, Bindings},
    session_work::RuntimeWork,
};
use blent_config::connection::{
    control::{Notice, State as LinkState},
    control_client::{self, SubmitError},
    control_view::{Action, Assembler, Limits, Model, View},
    pairing::{PeerId, Ticket},
    registration_client::{self, Phase},
    routes::RouteId,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn peer(id: u64) -> PeerId {
    PeerId::parse(format!("{id:064x}")).unwrap()
}
fn authenticated() -> View {
    let mut view = Assembler::new(Limits::default()).unwrap();
    view.push(Notice::Begin {
        revision: 1,
        local: peer(1),
    })
    .unwrap();
    view.push(Notice::Link {
        revision: 1,
        connection: 1,
        ticket: Some(Ticket {
            epoch: 1,
            peer: peer(2),
        }),
        state: LinkState::Authenticated,
        trusted: false,
    })
    .unwrap();
    Model::default()
        .publish(view.push(Notice::End { revision: 1 }).unwrap().unwrap())
        .unwrap()
}
#[derive(Clone)]
struct Source(View, Arc<AtomicBool>);
impl control_client::Client for Source {
    fn status(&self) -> control_client::Status {
        control_client::Status {
            phase: if self.1.load(Ordering::SeqCst) {
                control_client::Phase::Closed
            } else {
                control_client::Phase::Ready
            },
            view: Some(self.0.clone()),
            message: None,
            outcome: None,
        }
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {
        self.1.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.1.load(Ordering::SeqCst)
    }
}
impl control_client::Factory for Source {
    fn start(&mut self) -> Result<Box<dyn control_client::Client>> {
        Ok(Box::new(self.clone()))
    }
}
struct Broker {
    closed: AtomicBool,
}
impl registration_client::Client for Broker {
    fn status(&self) -> registration_client::Status {
        let closed = self.closed.load(Ordering::SeqCst);
        registration_client::Status {
            phase: if closed {
                Phase::Retired
            } else {
                Phase::Active
            },
            route: Some(RouteId {
                owner: 1,
                revision: 1,
            }),
            remote_retired: closed,
            failure: None,
        }
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}
struct Registration(Arc<AtomicUsize>);
impl registration_client::Factory for Registration {
    fn start(
        &mut self,
        request: registration_client::Request,
    ) -> Result<Box<dyn registration_client::Client>> {
        assert_eq!(
            request.resource,
            blent_config::connection::management::Resource::DisplaySession
        );
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Broker {
            closed: AtomicBool::new(false),
        }))
    }
}
struct Capture {
    manager: CaptureManager,
    participant: blent::display_quiescence::Participant,
    video: VideoOutput,
}
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, context: CaptureContext) -> CaptureWorkers {
        let Self {
            mut manager,
            participant,
            video,
        } = *self;
        manager.bind_display_work(Arc::new(
            blent::display_work::View::new(context.activity.clone(), Some(video.viewers()), true)
                .with_gate(participant.source()),
        ));
        manager.bind_quiescence(participant);
        manager.bind_activity(context.activity);
        manager.bind_profile(context.identity);
        manager.bind_retirement(context.retirement);
        CaptureWorkers {
            capture: Box::pin(async move {
                let result = manager
                    .stream_frames(video, context.settings_rx, context.display, context.stop)
                    .await;
                manager.shutdown().await;
                result
            }),
            auxiliary: vec![],
        }
    }
}
struct Fixture {
    connections: Connections,
    slots: Slots,
    settings: watch::Sender<EncoderSettings>,
    received: blent::video_queue::VideoReceiver,
    headers: CodecConfig,
    count: Arc<AtomicUsize>,
    source_closed: Arc<AtomicBool>,
    retirement: blent_config::retirement::Completion,
    _shutdown: watch::Sender<bool>,
    _listeners: blent::session_network::RetainedListeners,
}
impl Fixture {
    async fn new(source: watch::Receiver<Publication>) -> Self {
        let manager = streaming_manager();
        let headers = manager.codec_config();
        let (control, mut participants) = blent::display_quiescence::channel(1).unwrap();
        let (video, received) = crate::video_queue::channel(8, Default::default());
        let backend = Arc::new(crate::input::LinuxBackend::new(manager.card_rx()));
        let (mode, _) = watch::channel(false);
        let prepared = blent::session::Spec {
            settings: settings(&manager.config),
            instance: 0,
            ports: (0, 0),
            token: None,
            devices: (false, false, false),
        }
        .prepare(
            mode,
            blent::session::SessionResources {
                codec_config: headers.clone(),
                idr_wanted: manager.idr_request_flag(),
                latency: manager.latency_tracker(),
            },
            Box::new(Capture {
                manager,
                participant: participants.pop().unwrap(),
                video: video.into(),
            }),
            backend,
        )
        .with_quiescence(control.clone())
        .with_activity(Some(source.clone()));
        let settings = prepared.settings.clone();
        let (shutdown, stopped) = watch::channel(false);
        let (runtime, listeners) = prepared.start_exported(stopped).await.unwrap();
        let ports = listeners.ports();
        let now = Instant::now();
        let source_closed = Arc::new(AtomicBool::new(false));
        let discovery = Discovery::new(
            Box::new(Source(authenticated(), source_closed.clone())),
            Duration::from_secs(1),
            now,
        )
        .unwrap();
        let mut connections =
            Connections::new(discovery, vec![ports], Duration::from_secs(1)).unwrap();
        let mut slots = Slots::new(1).unwrap();
        connections.poll(&mut slots, now).unwrap();
        let Decision::Start(admission) = connections.plan(&mut slots, now).unwrap() else {
            panic!("T1016 original authenticated admission missing")
        };
        let count = Arc::new(AtomicUsize::new(0));
        let retirement = runtime.retirement.clone();
        connections.bind(
            &mut slots,
            admission,
            Bindings {
                activity: Some(ActivityBinding {
                    control,
                    retirement: runtime.retirement.clone(),
                    source: Some(source),
                }),
                attachment: runtime.tablet_tx.clone(),
                registration: Box::new(Registration(count.clone())),
                entropy: blent_config::credentials::system_entropy,
                work: Box::new(RuntimeWork::owned(runtime)),
            },
        );
        Self {
            connections,
            slots,
            settings,
            received,
            headers,
            count,
            source_closed,
            retirement,
            _shutdown: shutdown,
            _listeners: listeners,
        }
    }
    async fn active(&mut self, now: Instant) {
        timeout(Duration::from_secs(5), async {
            while self.connections.status().is_empty() {
                self.connections.poll(&mut self.slots, now).unwrap();
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        self.settings
            .send_modify(|settings| settings.geometry_ready = true);
    }
    async fn paused(&mut self, first: &crate::media::VideoPacket, now: Instant) {
        timeout(Duration::from_secs(5), async {
            loop {
                self.connections.poll(&mut self.slots, now).unwrap();
                if self.connections.status().is_empty()
                    && self.headers.current().is_none()
                    && !first.generation.load(Ordering::Acquire)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    async fn finish<T>(self, outcome: std::thread::Result<T>) -> std::thread::Result<T> {
        self.stop().await;
        outcome
    }
    async fn stop(mut self) {
        self.connections.close(&mut self.slots);
        timeout(Duration::from_secs(5), async {
            while !self.connections.retired() {
                self.connections
                    .poll(&mut self.slots, Instant::now())
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn t1016_registration_pause_waits_for_actual_native_generation_and_resumes_fresh_codec_keyframe(
) {
    if ownership_tests::isolated("capture::activity_tests::registered::t1016_registration_pause_waits_for_actual_native_generation_and_resumes_fresh_codec_keyframe") { return; }
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    let (published, source) = watch::channel(publication(&feed));
    let mut fixture = Fixture::new(source).await;
    let outcome = futures_util::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let now = Instant::now();
        fixture.active(now).await;
        let first = packet(&mut fixture.received).await;
        assert!(first.is_idr && first.codec_config.is_some());
        let root = crate::runtime::runtime_dir().unwrap();
        let helper = std::fs::read_to_string(root.join("helper.pid")).unwrap();
        observe(&mut feed, 2, true);
        published.send_replace(publication(&feed));
        fixture.paused(&first, now).await;
        assert_eq!(fixture.count.load(Ordering::SeqCst), 1);
        while fixture.received.try_recv().is_ok() {}
        assert!(timeout(Duration::from_millis(30), fixture.received.recv())
            .await
            .is_err());
        observe(&mut feed, 3, false);
        published.send_replace(publication(&feed));
        fixture.active(now + Duration::from_secs(2)).await;
        let fresh = packet(&mut fixture.received).await;
        assert_eq!(fixture.count.load(Ordering::SeqCst), 2);
        assert!(fresh.is_idr && fresh.codec_config.is_some());
        assert!(fresh.generation.load(Ordering::Acquire));
        assert!(!Arc::ptr_eq(&first.generation, &fresh.generation));
        assert_ne!(first.seq, fresh.seq);
        assert_eq!(
            std::fs::read_to_string(root.join("helper.pid")).unwrap(),
            helper
        );
    }))
    .await;
    let outcome = fixture.finish(outcome).await;
    if let Err(failure) = outcome {
        std::panic::resume_unwind(failure);
    }
}

#[path = "joined_activity_tests.rs"]
mod joined;
