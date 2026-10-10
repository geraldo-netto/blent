use super::*;
use blent_config::connection::{
    control::{Notice, State},
    control_client::{Client, Phase, Status, SubmitError},
    control_view::{Action, Assembler, Limits, Model},
    pairing::{PeerId, Ticket},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

fn peer(id: u64) -> PeerId {
    PeerId::parse(format!("{id:064x}")).unwrap()
}
struct Source(Arc<Mutex<Status>>, Arc<AtomicBool>);
impl Client for Source {
    fn status(&self) -> Status {
        self.0.lock().unwrap().clone()
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
impl blent::display_discovery::Factory for Source {
    fn start(&mut self) -> Result<Box<dyn Client>> {
        Ok(Box::new(Self(self.0.clone(), self.1.clone())))
    }
}
struct Fixture {
    state: Monitor,
    runtime: ExtraSession,
    source: Arc<Mutex<Status>>,
    model: Model,
    revision: u64,
    root: tempfile::TempDir,
    _shutdown: watch::Sender<bool>,
}
impl Fixture {
    async fn new(capacity: u32) -> Self {
        let mut state = super::super::tests::fixture();
        state.slots = Slots::new(capacity).unwrap();
        state.config.extra.max_tablets = capacity;
        let (shutdown, receiver) = watch::channel(false);
        state.config.extra.shutdown_rx = receiver;
        let (runtime, listeners) = session::Spec {
            capture: capture::CaptureConfig {
                helper_path: "/missing-t908-helper".into(),
                ..Default::default()
            },
            ports: (0, 0),
            token: None,
            devices: (false, false, false),
        }
        .prepare(state.config.extra.mode_tx.clone())
        .start_exported(state.config.extra.shutdown_rx.clone())
        .await
        .unwrap();
        state.config.ports = listeners.ports();
        state.config.extra.video_port = state.config.ports.0;
        state.config.extra.input_port = state.config.ports.1;
        state.config.tablet = runtime.tablet_tx.clone();
        let root = tempfile::tempdir().unwrap();
        let launch = Launch {
            executable: "/bin/false".into(),
            helper: "/bin/false".into(),
            runtime: root.path().join("runtime"),
            state: root.path().join("state"),
        };
        let mut native = Native::new(&state.config, &runtime, listeners, launch).unwrap();
        let source = Arc::new(Mutex::new(Status {
            phase: Phase::Ready,
            view: None,
            message: None,
            outcome: None,
        }));
        native.connections = Connections::new(
            Discovery::new(
                Box::new(Source(source.clone(), Arc::new(AtomicBool::new(false)))),
                Duration::from_secs(1),
                Instant::now(),
            )
            .unwrap(),
            config::slot_ports(state.config.ports.0, state.config.ports.1, capacity).unwrap(),
            Duration::from_secs(1),
        )
        .unwrap();
        state.config.usb = Some(native);
        Self {
            state,
            runtime,
            source,
            model: Model::default(),
            revision: 0,
            root,
            _shutdown: shutdown,
        }
    }
    fn links(&mut self, links: &[(u64, u64)]) {
        self.revision += 1;
        let mut assembler = Assembler::new(Limits::default()).unwrap();
        assembler
            .push(Notice::Begin {
                revision: self.revision,
                local: peer(1),
            })
            .unwrap();
        for &(connection, identity) in links {
            assembler
                .push(Notice::Link {
                    revision: self.revision,
                    connection,
                    ticket: Some(Ticket {
                        epoch: 1,
                        peer: peer(identity),
                    }),
                    state: State::Authenticated,
                    trusted: false,
                })
                .unwrap();
        }
        let view = self
            .model
            .publish(
                assembler
                    .push(Notice::End {
                        revision: self.revision,
                    })
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
        self.source.lock().unwrap().view = Some(view);
    }
    async fn complete(&mut self) {
        let native = self.state.config.usb.as_mut().unwrap();
        while native.starting() {
            let result = tokio::time::timeout(Duration::from_secs(2), native.next())
                .await
                .unwrap();
            self.state.usb_completed(result);
            break;
        }
    }
    async fn stop(self) {
        tokio::time::timeout(Duration::from_secs(3), self.state.stop())
            .await
            .unwrap();
        self.runtime.stop().await;
    }
}
#[tokio::test]
async fn t908_primary_handoff_rejects_unrelated_listener_capabilities() {
    let fixture = Fixture::new(1).await;
    let prepared = session::Spec {
        capture: Default::default(),
        ports: (0, 0),
        token: None,
        devices: (false, false, false),
    }
    .prepare(fixture.state.config.extra.mode_tx.clone());
    let (other, listeners) = prepared
        .start_exported(fixture.state.config.extra.shutdown_rx.clone())
        .await
        .unwrap();
    let launch = Launch {
        executable: "/bin/false".into(),
        helper: "/bin/false".into(),
        runtime: fixture.root.path().join("wrong-runtime"),
        state: fixture.root.path().join("wrong-state"),
    };
    assert!(
        Native::new(&fixture.state.config, &fixture.runtime, listeners, launch).is_err(),
        "T908 mismatched primary capabilities accepted"
    );
    other.stop().await;
    fixture.stop().await;
}
#[tokio::test]
async fn t908_primary_and_extra_start_independently_and_retain_real_ports() {
    let mut fixture = Fixture::new(2).await;
    fixture.links(&[(1, 2), (2, 3)]);
    fixture.state.usb_progress();
    assert_eq!(fixture.state.config.usb.as_ref().unwrap().startups.len(), 2);
    fixture.complete().await;
    fixture.complete().await;
    for instance in 0..2 {
        let ports = config::slot_ports(
            fixture.state.config.ports.0,
            fixture.state.config.ports.1,
            2,
        )
        .unwrap()[instance];
        for port in [ports.0, ports.1] {
            assert!(
                std::net::TcpListener::bind(("127.0.0.1", port)).is_err(),
                "T908 native listener ownership lost"
            );
        }
    }
    assert!(!*fixture.runtime.stop_tx.borrow());
    fixture.links(&[]);
    fixture.state.usb_progress();
    tokio::time::timeout(Duration::from_secs(2), async {
        while fixture
            .state
            .slots
            .ticket(&DisplayOwner::Peer(peer(2)))
            .is_some()
            || fixture
                .state
                .slots
                .ticket(&DisplayOwner::Peer(peer(3)))
                .is_some()
        {
            fixture.state.usb_progress();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(!*fixture.runtime.stop_tx.borrow());
    fixture.stop().await;
}
#[tokio::test]
async fn t908_selected_usb_preempts_adb_after_cleanup_and_ambiguity_preserves_it() {
    let mut fixture = Fixture::new(1).await;
    fixture.state.change_primary(&Some("fake:t908".into()));
    fixture.state.prepare_primary();
    let ticket = fixture
        .state
        .slots
        .ticket(&DisplayOwner::Adb("fake:t908".into()))
        .unwrap();
    fixture.links(&[(1, 2), (2, 2)]);
    fixture.state.usb_progress();
    assert_eq!(fixture.state.slots.closing(ticket), Some(false));
    fixture.links(&[(1, 2)]);
    fixture.state.usb_progress();
    assert!(fixture.state.current.is_none());
    assert_eq!(fixture.state.slots.closing(ticket), Some(true));
    assert!(!fixture.state.config.usb.as_ref().unwrap().starting());
    let retired = fixture.state.retiring.join_next().await.unwrap().unwrap();
    fixture.state.retired(retired);
    fixture.state.usb_progress();
    fixture.complete().await;
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(2)))
        .is_some());
    fixture.state.change_primary(&Some("another-adb".into()));
    assert!(fixture.state.current.is_none());
    fixture.stop().await;
}
#[tokio::test]
async fn t908_bind_failure_releases_only_unpublished_start_and_retries_later() {
    let mut fixture = Fixture::new(2).await;
    let ports = config::slot_ports(
        fixture.state.config.ports.0,
        fixture.state.config.ports.1,
        2,
    )
    .unwrap()[1];
    let held = std::net::TcpListener::bind(("127.0.0.1", ports.1)).unwrap();
    fixture.links(&[(1, 2), (2, 3)]);
    fixture.state.usb_progress();
    fixture.complete().await;
    fixture.complete().await;
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(2)))
        .is_none());
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(3)))
        .is_some());
    assert!(std::net::TcpListener::bind(("127.0.0.1", ports.0)).is_ok());
    drop(held);
    fixture.state.usb_progress();
    assert!(!fixture.state.config.usb.as_ref().unwrap().starting());
    fixture.stop().await;
}

struct Registered(Arc<AtomicBool>);
impl blent_config::connection::registration_client::Client for Registered {
    fn status(&self) -> blent_config::connection::registration_client::Status {
        use blent_config::connection::{
            registration_client::{Phase, Status},
            routes::RouteId,
        };
        Status {
            phase: if self.0.load(Ordering::SeqCst) {
                Phase::Retired
            } else {
                Phase::Active
            },
            route: Some(RouteId {
                owner: 1,
                revision: 1,
            }),
            remote_retired: self.0.load(Ordering::SeqCst),
            failure: None,
        }
    }
    fn close(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
struct Registration;
impl blent::registered_display::Registration for Registration {
    fn start(
        &mut self,
        _: blent_config::connection::registration_client::Request,
    ) -> Result<Box<dyn blent_config::connection::registration_client::Client>> {
        Ok(Box::new(Registered(Arc::new(AtomicBool::new(false)))))
    }
}
#[tokio::test]
async fn t908_active_native_status_and_usb_steps_do_not_queue_adb_inventory() {
    let mut fixture = Fixture::new(1).await;
    fixture.links(&[(1, 2)]);
    let native = fixture.state.config.usb.as_mut().unwrap();
    native
        .connections
        .poll(&mut fixture.state.slots, Instant::now())
        .unwrap();
    let Decision::Start(admission) = native
        .connections
        .plan(&mut fixture.state.slots, Instant::now())
        .unwrap()
    else {
        panic!("T908 admission missing")
    };
    let bindings = Bindings {
        attachment: fixture.runtime.tablet_tx.clone(),
        registration: Box::new(Registration),
        entropy: blent_config::credentials::system_entropy,
        work: Box::new(RuntimeWork::shared(&fixture.runtime)),
    };
    let mut ledger =
        Some(runtime::SessionLedger::new(fixture.root.path().join("sessions.json")).unwrap());
    let event = Event::UsbStartup(Some(Ok((admission, Ok(bindings)))));
    assert!(fixture.state.step(event, &mut ledger).await);
    assert!(fixture.state.step(Event::UsbTick, &mut ledger).await);
    assert!(!fixture.state.checked_adb);
    assert!(fixture.state.inventory.is_empty());
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("sessions.json")).unwrap())
            .unwrap();
    let sessions: Vec<runtime::TabletSession> =
        serde_json::from_value(snapshot["sessions"].clone()).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].identity.peer(), Some(&peer(2)));
    assert_eq!(
        (sessions[0].video_port, sessions[0].input_port),
        fixture.state.config.ports
    );
    fixture.links(&[]);
    fixture.state.step(Event::UsbTick, &mut ledger).await;
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.root.path().join("sessions.json")).unwrap())
            .unwrap();
    assert!(snapshot["sessions"].as_array().unwrap().is_empty());
    assert!(matches!(
        fixture.state.usb_event(Event::Tick),
        Some(Event::Tick)
    ));
    assert!(!fixture.state.step(Event::Stop, &mut ledger).await);
    fixture.stop().await;
}
#[tokio::test]
async fn t908_native_event_has_bounded_ticks_and_independent_startup_completion() {
    let mut fixture = Fixture::new(1).await;
    fixture.links(&[(1, 2)]);
    fixture.state.usb_progress();
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.tick().await;
    let completion = tokio::time::timeout(
        Duration::from_secs(2),
        event(&mut fixture.state.config.usb, &mut interval),
    )
    .await
    .unwrap();
    assert!(matches!(completion, Event::UsbStartup(Some(Ok(_)))));
    fixture.state.usb_event(completion);
    let mut interval = tokio::time::interval(Duration::from_millis(1));
    assert!(matches!(
        event(&mut fixture.state.config.usb, &mut interval).await,
        Event::UsbTick
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(10), event(&mut None, &mut interval))
            .await
            .is_err()
    );
    fixture.state.usb_completed(None);
    fixture.stop().await;
}
#[tokio::test]
async fn t908_invalid_extra_instance_rolls_back_before_native_work() {
    let fixture = Fixture::new(1).await;
    for instance in [1, 2, 4, 128, u32::MAX] {
        assert!(prepare_extra_session(&fixture.state.config.extra, instance).is_err());
    }
    fixture.stop().await;
}

#[tokio::test]
async fn t908_stuck_startup_does_not_stall_another_peer_and_late_cancel_is_owned() {
    let mut fixture = Fixture::new(2).await;
    fixture.links(&[(1, 2), (2, 3)]);
    let native = fixture.state.config.usb.as_mut().unwrap();
    native
        .connections
        .poll(&mut fixture.state.slots, Instant::now())
        .unwrap();
    let Decision::Start(admission) = native
        .connections
        .plan(&mut fixture.state.slots, Instant::now())
        .unwrap()
    else {
        panic!("T908 admission missing")
    };
    let (release, hold) = tokio::sync::oneshot::channel();
    native.startups.spawn(async move {
        hold.await.unwrap();
        (
            admission,
            Err(anyhow::anyhow!("T908 unpublished cancelled startup")),
        )
    });
    fixture.state.usb_progress();
    fixture.complete().await;
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(2)))
        .is_some());
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(3)))
        .is_some());
    assert_eq!(fixture.state.config.usb.as_ref().unwrap().startups.len(), 1);
    fixture.links(&[(1, 2)]);
    fixture.state.usb_progress();
    let ticket = fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(3)))
        .unwrap();
    assert_eq!(fixture.state.slots.closing(ticket), Some(true));
    release.send(()).unwrap();
    fixture.complete().await;
    assert!(fixture.state.slots.owner(ticket).is_none());
    fixture.stop().await;
}
#[tokio::test]
async fn t908_panicked_startup_quarantines_capacity_and_other_peer_still_progresses() {
    let mut fixture = Fixture::new(2).await;
    fixture.links(&[(1, 2), (2, 3)]);
    let native = fixture.state.config.usb.as_mut().unwrap();
    native
        .connections
        .poll(&mut fixture.state.slots, Instant::now())
        .unwrap();
    let Decision::Start(admission) = native
        .connections
        .plan(&mut fixture.state.slots, Instant::now())
        .unwrap()
    else {
        panic!("T908 admission missing")
    };
    native.startups.spawn(async move {
        drop(admission);
        panic!("T908 startup death");
    });
    let result = native.next().await.unwrap();
    assert!(result.is_err());
    native.completed(&mut fixture.state.slots, result);
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(3)))
        .is_some());
    fixture.state.usb_progress();
    fixture.complete().await;
    assert!(fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(2)))
        .is_some());
    // T928: startup exit alone cannot establish native cleanup. Preserve the
    // quarantine instead of releasing it; the fixture created no native work.
    let mut native = fixture.state.config.usb.take().unwrap();
    native.connections.close(&mut fixture.state.slots);
    assert!(!native.connections.retired());
    drop(native);
    fixture.stop().await;
}

#[tokio::test]
async fn t908_shutdown_joins_pending_startup_without_admitting_late_binding() {
    let mut fixture = Fixture::new(2).await;
    fixture.links(&[(1, 2), (2, 3)]);
    fixture.state.usb_progress();
    assert_eq!(fixture.state.config.usb.as_ref().unwrap().startups.len(), 2);
    fixture.stop().await;
}
