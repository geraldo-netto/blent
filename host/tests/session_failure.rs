//! T813: a terminal session worker must retire only its own session.
use blent::{
    capture_contract::*,
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
use tokio::sync::{oneshot, watch};

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
struct Complete(Arc<AtomicBool>);
impl Drop for Complete {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
struct Capture {
    release: oneshot::Receiver<()>,
    outcome: u8,
    started: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
}
impl CaptureBackend for Capture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        let Self {
            release,
            outcome,
            started,
            done,
        } = *self;
        CaptureWorkers {
            capture: Box::pin(async move {
                let _ = release.await;
                match outcome {
                    0 => Ok(()),
                    1 => anyhow::bail!("T813 capture failure"),
                    _ => panic!("T813 capture panic"),
                }
            }),
            auxiliary: vec![Box::pin(async move {
                let _complete = Complete(done);
                started.store(true, Ordering::Release);
                let _ = context.stop.wait_for(|stop| *stop).await;
                Ok(())
            })],
        }
    }
}
fn spec(instance: u32) -> Spec {
    Spec {
        instance,
        ports: (0, 0),
        token: None,
        devices: (false, false, false),
        settings: EncoderSettings {
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
        },
    }
}
async fn failure(outcome: u8) {
    let (trigger, release) = oneshot::channel();
    let started = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let prepared = spec(0).prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(Capture {
            release,
            outcome,
            started: started.clone(),
            done: done.clone(),
        }),
        Arc::new(Input),
    );
    prepared.tablet.begin(Some("device:T813".into()));
    let lease = prepared.tablet.lease();
    let (_daemon, stop) = watch::channel(false);
    let runtime = prepared.start(stop).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !started.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    trigger.send(()).unwrap();
    let retired =
        tokio::time::timeout(Duration::from_millis(150), lease.retirement().retired()).await;
    assert!(
        retired.is_ok(),
        "T813: terminal worker left attachment generation active (outcome {outcome})"
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while !done.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("T813: sibling worker was not retired");
    assert!(
        *runtime.stop_tx.borrow(),
        "T813: failure was not propagated to the session stop signal"
    );
    runtime.stop().await;
}
#[tokio::test]
async fn t813_capture_error_retires_attachment_and_siblings() {
    failure(1).await;
}
#[tokio::test]
async fn t813_capture_panic_retires_attachment_and_siblings() {
    failure(2).await;
}
#[tokio::test]
async fn t813_unexpected_capture_success_retires_attachment_and_siblings() {
    failure(0).await;
}

struct StartupPanic;
impl CaptureBackend for StartupPanic {
    fn start(self: Box<Self>, _: CaptureContext) -> CaptureWorkers {
        panic!("T813 startup panic");
    }
}
#[tokio::test]
async fn t813_startup_panic_returns_error_and_releases_listeners() {
    use tokio::net::TcpListener;
    let first = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let second = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ports = (
        first.local_addr().unwrap().port(),
        second.local_addr().unwrap().port(),
    );
    drop((first, second));
    let mut config = spec(0);
    config.ports = ports;
    let prepared = config.prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(StartupPanic),
        Arc::new(Input),
    );
    let (_daemon, stop) = watch::channel(false);
    let task = tokio::spawn(prepared.start(stop));
    let result = task
        .await
        .expect("T813: factory panic escaped startup boundary");
    assert!(result.is_err());
    let _first = TcpListener::bind(("127.0.0.1", ports.0))
        .await
        .expect("T813: leaked stream listener");
    let _second = TcpListener::bind(("127.0.0.1", ports.1))
        .await
        .expect("T813: leaked input listener");
}
fn controlled(
    instance: u32,
    outcome: u8,
    name: &'static str,
) -> (blent::session::Runtime, oneshot::Sender<()>) {
    use blent::{
        attachment::Attachment,
        session::{Runtime, Workers},
    };
    let (settings, _) = watch::channel(spec(instance).settings);
    let tablet = Attachment::with_token(settings, None);
    tablet.begin(Some(format!("device:{instance}")));
    let (stop_tx, mut stop) = watch::channel(false);
    let admission = Workers::reserve().unwrap();
    let capture = tokio::spawn(async move {
        let _ = stop.wait_for(|stop| *stop).await;
        Ok(())
    });
    let (trigger, release) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = release.await;
        match outcome {
            0 => Ok(()),
            1 => anyhow::bail!("T813 service error"),
            _ => panic!("T813 service panic"),
        }
    });
    (
        Runtime::from_workers(
            instance,
            tablet,
            Arc::new(tokio::sync::Notify::new()),
            stop_tx,
            Workers::new(admission, capture, vec![(name, task)]),
            (0, 0),
        ),
        trigger,
    )
}
#[tokio::test]
async fn t813_owner_drop_closes_status_with_terminal_value() {
    use blent::session_state::State;
    let (runtime, _trigger) = controlled(0, 0, "Stream");
    let mut status = runtime.status.clone();
    drop(runtime);
    tokio::task::yield_now().await;
    assert_ne!(
        *status.borrow_and_update(),
        State::Running,
        "T813: dropped session still reports running"
    );
    assert!(
        status.changed().await.is_err(),
        "T813: dropped supervisor retained status owner"
    );
}
#[tokio::test]
async fn t813_service_outcomes_publish_failure_without_stopping_other_sessions() {
    use blent::session_state::State;
    for name in ["Stream", "Input", "Capture auxiliary"] {
        for outcome in 0..3 {
            let (failed, trigger) = controlled(0, outcome, name);
            let (healthy, _held) = controlled(1, 0, "Stream");
            let old = failed.tablet_tx.lease();
            let healthy_lease = healthy.tablet_tx.lease();
            let mut status = failed.status.clone();
            trigger.send(()).unwrap();
            tokio::time::timeout(
                Duration::from_secs(2),
                status.wait_for(|state| matches!(state, State::Failed(_))),
            )
            .await
            .unwrap()
            .unwrap();
            let failure = match &*status.borrow() {
                State::Failed(failure) => failure.clone(),
                state => panic!("{state:?}"),
            };
            assert_eq!(failure.worker, name);
            assert!(!failure.message.is_empty());
            assert!(
                !old.apply(|| ()),
                "T813: failed generation still accepted work"
            );
            assert!(healthy_lease.apply(|| ()));
            assert_eq!(*healthy.status.borrow(), State::Running);
            assert!(!*healthy.stop_tx.borrow());
            failed.stop().await;
            assert_eq!(*status.borrow(), State::Failed(failure));
            let healthy_status = healthy.status.clone();
            healthy.stop().await;
            assert_eq!(*healthy_status.borrow(), State::Stopped);
        }
    }
}

#[tokio::test]
async fn t813_failed_session_cannot_be_reactivated_by_late_transport() {
    let (runtime, trigger) = controlled(0, 1, "Stream");
    let mut status = runtime.status.clone();
    trigger.send(()).unwrap();
    tokio::time::timeout(
        Duration::from_secs(2),
        status.wait_for(|state| matches!(state, blent::session_state::State::Failed(_))),
    )
    .await
    .unwrap()
    .unwrap();
    for identity in [None, Some("device:late".into()), Some("device:0".into())] {
        runtime.tablet_tx.begin(identity);
        assert!(
            runtime.tablet_tx.send(true).is_err(),
            "T813: late transport restored failed session"
        );
        assert!(
            runtime.tablet_tx.token().is_err(),
            "T813: failed session issued credentials"
        );
        let lease = runtime.tablet_tx.lease();
        assert!(!lease.apply(|| panic!("T813: failed session accepted new work")));
        assert!(lease.token(None).is_err());
        tokio::time::timeout(Duration::from_millis(150), lease.retirement().retired())
            .await
            .unwrap();
    }
    runtime.stop().await;
}
struct TerminalInput(bool);
impl InputBackend for TerminalInput {
    fn sink(&self) -> Arc<dyn InputSink> {
        Arc::new(Input)
    }
    fn follow(
        &self,
        _: watch::Receiver<bool>,
        _: watch::Receiver<bool>,
        _: InputConfig,
    ) -> Signal<'static> {
        let panic = self.0;
        Box::pin(async move {
            assert!(!panic, "T813: native input panic");
        })
    }
}
#[tokio::test]
async fn t813_native_input_completion_is_a_session_failure() {
    for panic in [false, true] {
        let (_hold, release) = oneshot::channel();
        let prepared = spec(0).prepare(
            watch::channel(false).0,
            SessionResources::default(),
            Box::new(Capture {
                release,
                outcome: 0,
                started: Arc::new(AtomicBool::new(false)),
                done: Arc::new(AtomicBool::new(false)),
            }),
            Arc::new(TerminalInput(panic)),
        );
        let (_daemon, stop) = watch::channel(false);
        let runtime = prepared.start(stop).await.unwrap();
        let mut status = runtime.status.clone();
        tokio::time::timeout(
            Duration::from_millis(150),
            status.wait_for(|state| matches!(state, blent::session_state::State::Failed(_))),
        )
        .await
        .expect("T813: native input completion erased")
        .unwrap();
        assert!(
            matches!(&*status.borrow(), blent::session_state::State::Failed(failure) if failure.worker == "Input")
        );
        runtime.stop_tx.send_replace(true);
        drop(runtime);
    }
}

#[tokio::test(start_paused = true)]
async fn t813_normal_shutdown_aborts_unresponsive_async_capture_after_grace() {
    use blent::{
        attachment::Attachment,
        session::{Runtime, Workers},
        session_state::State,
    };
    let tablet = Attachment::new(watch::channel(spec(0).settings).0);
    let done = Arc::new(AtomicBool::new(false));
    let guard = Complete(done.clone());
    let admission = Workers::reserve().unwrap();
    let capture = tokio::spawn(async move {
        let _guard = guard;
        std::future::pending::<anyhow::Result<()>>().await
    });
    let runtime = Runtime::from_workers(
        0,
        tablet,
        Arc::new(tokio::sync::Notify::new()),
        watch::channel(false).0,
        Workers::new(admission, capture, vec![]),
        (0, 0),
    );
    let status = runtime.status.clone();
    runtime.stop().await;
    assert!(done.load(Ordering::Acquire));
    assert_eq!(*status.borrow(), State::Stopped);
}
#[tokio::test]
async fn t813_already_requested_daemon_shutdown_is_not_failure() {
    use blent::session_state::State;
    for closed in [false, true] {
        let (_trigger, release) = oneshot::channel();
        let prepared = spec(0).prepare(
            watch::channel(false).0,
            SessionResources::default(),
            Box::new(Capture {
                release,
                outcome: 0,
                started: Arc::new(AtomicBool::new(false)),
                done: Arc::new(AtomicBool::new(false)),
            }),
            Arc::new(Input),
        );
        let (daemon, stop) = watch::channel(!closed);
        if closed {
            drop(daemon);
        }
        let runtime = prepared.start(stop).await.unwrap();
        tokio::task::yield_now().await;
        assert!(*runtime.stop_tx.borrow());
        assert!(!matches!(&*runtime.status.borrow(), State::Failed(_)));
        drop(runtime);
    }
}
#[tokio::test]
async fn t813_retirement_rejects_bounded_invalid_and_late_transport_values() {
    use blent::attachment::Attachment;
    for identity in [
        "",
        "device:",
        "device:\0",
        "device:é",
        "unknown",
        &"x".repeat(4096),
    ] {
        for token in [None, Some("".into()), Some("token".into())] {
            let tablet = Attachment::with_token(watch::channel(spec(0).settings).0, token);
            let presence = tablet.subscribe();
            tablet.begin(Some(identity.into()));
            let old = tablet.lease();
            tablet.retire();
            tablet.retire();
            for present in [false, true] {
                assert!(tablet.send(present).is_err());
            }
            tablet.begin_with_transport(
                Some(identity.into()),
                Some(blent_config::adb::Transport::Network),
            );
            assert!(!*presence.borrow());
            assert!(tablet.token().is_err());
            assert!(!old.apply(|| panic!("retired action")));
            let new = tablet.lease();
            assert!(!new.apply(|| panic!("late action")));
            assert!(new.profile_identity().is_none());
            assert!(new.transport().is_none());
            assert!(new.token(Some("fallback")).is_err());
            tokio::time::timeout(Duration::from_millis(150), old.retirement().retired())
                .await
                .unwrap();
            tokio::time::timeout(Duration::from_millis(150), new.retirement().retired())
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn t813_worker_panic_during_generation_action_still_retires_session() {
    use blent::{
        attachment::Attachment,
        session::{Runtime, Workers},
        session_state::State,
    };
    let tablet = Attachment::new(watch::channel(spec(0).settings).0);
    tablet.begin(Some("device:panicking-action".into()));
    let lease = tablet.lease();
    let admission = Workers::reserve().unwrap();
    let capture = tokio::spawn(async move {
        lease.apply(|| panic!("T813 action panic"));
        Ok(())
    });
    let runtime = Runtime::from_workers(
        0,
        tablet,
        Arc::new(tokio::sync::Notify::new()),
        watch::channel(false).0,
        Workers::new(admission, capture, vec![]),
        (0, 0),
    );
    let mut status = runtime.status.clone();
    let failure = tokio::time::timeout(
        Duration::from_millis(150),
        status.wait_for(|state| matches!(state, State::Failed(_))),
    )
    .await;
    // Preserve a single assertion failure on the pre-fix poisoned-lock path.
    if failure.is_err() {
        std::mem::forget(runtime);
        panic!("T813: worker panic also crashed session supervisor");
    }
    drop(failure);
    assert!(runtime.tablet_tx.token().is_err());
    assert!(!runtime.tablet_tx.lease().apply(|| panic!("retired work")));
    runtime.stop().await;
}

#[tokio::test]
async fn t912_ephemeral_session_reports_actual_owned_listener_ports() {
    let (trigger, release) = oneshot::channel();
    let prepared = spec(0).prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(Capture {
            release,
            outcome: 0,
            started: Arc::new(AtomicBool::new(false)),
            done: Arc::new(AtomicBool::new(false)),
        }),
        Arc::new(Input),
    );
    let (_daemon, stop) = watch::channel(false);
    let runtime = prepared.start(stop).await.unwrap();
    assert_ne!(
        runtime.video_port, 0,
        "T912 must report the bound video port"
    );
    assert_ne!(
        runtime.input_port, 0,
        "T912 must report the bound control port"
    );
    assert_ne!(runtime.video_port, runtime.input_port);
    assert!(std::net::TcpListener::bind(("127.0.0.1", runtime.video_port)).is_err());
    assert!(std::net::TcpListener::bind(("127.0.0.1", runtime.input_port)).is_err());
    trigger.send(()).unwrap();
    runtime.stop().await;
}

#[tokio::test]
async fn t912_exported_listener_capabilities_outlive_actual_session_retirement() {
    let (trigger, release) = oneshot::channel();
    let prepared = spec(0).prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(Capture {
            release,
            outcome: 0,
            started: Arc::new(AtomicBool::new(false)),
            done: Arc::new(AtomicBool::new(false)),
        }),
        Arc::new(Input),
    );
    let (_daemon, stop) = watch::channel(false);
    let (runtime, retained) = prepared.start_exported(stop).await.unwrap();
    let ports = retained.ports();
    assert_eq!(ports, (runtime.video_port, runtime.input_port));
    let native = retained.try_clone().unwrap();
    assert_eq!(native.len(), 2);
    assert_eq!(native[0].local_addr().unwrap().port(), ports.0);
    assert_eq!(native[1].local_addr().unwrap().port(), ports.1);
    trigger.send(()).unwrap();
    let retirement = runtime.stop().await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while !retirement.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
    }
    drop(retained);
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
    }
    drop(native);
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}

#[tokio::test]
async fn t912_partial_bind_failure_rolls_back_before_capture_or_endpoint_publication() {
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let vacant = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let video_port = vacant.local_addr().unwrap().port();
    drop(vacant);
    let mut config = spec(0);
    config.ports = (video_port, occupied.local_addr().unwrap().port());
    let prepared = config.prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(StartupPanic),
        Arc::new(Input),
    );
    let (_daemon, stop) = watch::channel(false);
    let error = match prepared.start_exported(stop).await {
        Ok(_) => panic!("T912 occupied input endpoint was accepted"),
        Err(error) => error,
    };
    assert!(!error.to_string().contains("Capture startup panicked"));
    assert!(std::net::TcpListener::bind(("127.0.0.1", video_port)).is_ok());
}

#[tokio::test]
async fn t912_legacy_server_start_keeps_its_listeners_after_export_is_dropped() {
    let reserved = [
        std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
        std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
    ];
    let ports = (
        reserved[0].local_addr().unwrap().port(),
        reserved[1].local_addr().unwrap().port(),
    );
    drop(reserved);
    let mut config = spec(0);
    config.ports = ports;
    let prepared = config.prepare(
        watch::channel(false).0,
        SessionResources::default(),
        Box::new(StartupPanic),
        Arc::new(Input),
    );
    let (video, _) = blent::video_queue::channel(8, Default::default());
    let (stream, input) = blent::session::start_servers(prepared.stream, prepared.input, video)
        .await
        .unwrap();
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_err());
    }
    stream.abort();
    input.abort();
    let _ = tokio::join!(stream, input);
    for port in [ports.0, ports.1] {
        assert!(std::net::TcpListener::bind(("127.0.0.1", port)).is_ok());
    }
}
