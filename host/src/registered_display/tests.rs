use super::*;
use crate::media::EncoderSettings;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use tokio::sync::watch;

struct Native {
    status: Mutex<Status>,
    closed: AtomicBool,
    retired: AtomicBool,
}
#[derive(Clone)]
struct Stub(Arc<Native>);
impl Stub {
    fn new() -> Self {
        Self(Arc::new(Native {
            status: Mutex::new(status(Phase::Starting, None)),
            closed: AtomicBool::new(false),
            retired: AtomicBool::new(false),
        }))
    }
    fn publish(&self, phase: Phase, route: Option<RouteId>) {
        *self.0.status.lock().unwrap() = status(phase, route);
    }
}
impl Client for Stub {
    fn status(&self) -> Status {
        self.0.status.lock().unwrap().clone()
    }
    fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.0.retired.load(Ordering::SeqCst)
    }
}
#[derive(Default)]
struct Calls {
    requests: Vec<Request>,
    clients: Vec<Stub>,
    fail: bool,
}
struct Backend(Arc<Mutex<Calls>>);
impl Registration for Backend {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>> {
        let mut calls = self.0.lock().unwrap();
        calls.requests.push(request);
        ensure!(!calls.fail, "injected unpublished startup failure");
        let client = Stub::new();
        calls.clients.push(client.clone());
        Ok(Box::new(client))
    }
}
fn status(phase: Phase, route: Option<RouteId>) -> Status {
    Status {
        phase,
        route,
        failure: None,
        remote_retired: phase == Phase::Retired,
    }
}
fn route(owner: u64) -> RouteId {
    RouteId { owner, revision: 1 }
}
fn entropy(bytes: &mut [u8]) -> Result<()> {
    static NEXT: AtomicU8 = AtomicU8::new(1);
    bytes.fill(NEXT.fetch_add(1, Ordering::SeqCst));
    Ok(())
}
fn peer(value: u8) -> PeerId {
    PeerId::parse(format!("{value:02x}").repeat(32)).unwrap()
}
struct Fixture {
    display: Display,
    attachment: Attachment,
    calls: Arc<Mutex<Calls>>,
    settings: watch::Sender<EncoderSettings>,
}
impl Fixture {
    fn new() -> Self {
        let (settings, _) = watch::channel(EncoderSettings {
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
        });
        let calls = Arc::new(Mutex::new(Calls::default()));
        let attachment = Attachment::new(settings.clone());
        let display = Display::new(
            peer(1),
            attachment.clone(),
            Box::new(Backend(calls.clone())),
            entropy,
            Duration::from_secs(1),
        )
        .unwrap();
        Self {
            display,
            attachment,
            calls,
            settings,
        }
    }
    fn client(&self) -> Stub {
        self.calls.lock().unwrap().clients.last().unwrap().clone()
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().requests.len()
    }
    fn active(&mut self, now: Instant) -> Stub {
        self.display.poll(now).unwrap();
        let client = self.client();
        client.publish(Phase::Active, Some(route(1)));
        self.display.poll(now).unwrap();
        assert_eq!(self.display.state(), State::Active);
        client
    }
}
#[test]
fn t920_renewal_waits_for_native_and_remote_retirement_and_preserves_geometry() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    let presence = fixture.attachment.subscribe();
    fixture.display.poll(now).unwrap();
    assert_eq!(fixture.display.state(), State::Registering);
    let client = fixture.client();
    client.publish(Phase::Queued, Some(route(1)));
    fixture.display.poll(now).unwrap();
    assert!(!*presence.borrow());
    client.publish(Phase::Active, Some(route(1)));
    fixture.display.poll(now).unwrap();
    assert!(*presence.borrow());
    fixture.settings.send_modify(|settings| {
        settings.width = 1920;
        settings.geometry_ready = true;
    });
    let accepted = fixture.attachment.lease();
    client.publish(Phase::Closing, Some(route(1)));
    fixture.display.poll(now).unwrap();
    assert_eq!(fixture.display.state(), State::Retiring);
    assert!(!accepted.apply(|| ()));
    assert!(*presence.borrow());
    client.publish(Phase::Retired, Some(route(1)));
    fixture.display.poll(now + Duration::from_secs(10)).unwrap();
    assert_eq!(fixture.count(), 1);
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now + Duration::from_secs(10)).unwrap();
    assert_eq!(fixture.display.state(), State::Waiting);
    fixture
        .display
        .poll(now + Duration::from_millis(10999))
        .unwrap();
    assert_eq!(fixture.count(), 1);
    fixture.display.poll(now + Duration::from_secs(11)).unwrap();
    assert_eq!(fixture.count(), 2);
    let calls = fixture.calls.lock().unwrap();
    assert!(calls.requests[0].credential != calls.requests[1].credential);
    assert_eq!(calls.requests[1].peer, peer(1));
    assert_eq!(calls.requests[1].resource, Resource::DisplaySession);
    assert_eq!(calls.requests[1].profile, Profile::Display);
    drop(calls);
    assert_eq!(fixture.settings.borrow().width, 1920);
    assert!(fixture.settings.borrow().geometry_ready);
    fixture.display.close();
    assert!(!*presence.borrow());
    fixture.client().publish(Phase::Active, Some(route(2)));
    fixture.display.poll(now + Duration::from_secs(20)).unwrap();
    assert!(!*presence.borrow());
    assert_eq!(fixture.count(), 2);
    fixture.client().0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now + Duration::from_secs(20)).unwrap();
    assert_eq!(fixture.display.state(), State::Retired);
}
#[test]
fn t920_late_close_does_not_revoke_a_replacement_attachment() {
    let mut fixture = Fixture::new();
    let client = fixture.active(Instant::now());
    let replacement = fixture
        .attachment
        .register_route(peer(2), Credential::new([42; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let lease = fixture.attachment.lease();
    fixture.display.close();
    assert!(client.0.closed.load(Ordering::SeqCst));
    assert!(lease.apply(|| ()), "T920 old owner invalidated replacement");
    assert!(*fixture.attachment.subscribe().borrow());
}
#[test]
fn t920_lost_authority_does_not_retry_over_its_replacement() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    fixture.display.poll(now).unwrap();
    let replacement = fixture
        .attachment
        .register_route(peer(2), Credential::new([43; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let lease = fixture.attachment.lease();
    let client = fixture.client();
    client.publish(Phase::Active, Some(route(1)));
    fixture.display.poll(now).unwrap();
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now).unwrap();
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(
        fixture.count(),
        1,
        "T920 stale owner registered over replacement"
    );
    drop(fixture.display);
    assert!(lease.apply(|| ()));
}
#[test]
fn t920_startup_and_entropy_errors_obey_absolute_retry_deadlines() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    fixture.calls.lock().unwrap().fail = true;
    assert!(fixture.display.poll(now).is_err());
    assert_eq!(fixture.count(), 1);
    assert!(fixture.attachment.token().is_err());
    for delay in [0, 1, 50, 999] {
        fixture
            .display
            .poll(now + Duration::from_millis(delay))
            .unwrap();
    }
    assert_eq!(fixture.count(), 1);
    fixture.calls.lock().unwrap().fail = false;
    fixture.display.entropy = |_| anyhow::bail!("entropy unavailable");
    assert!(fixture.display.poll(now + Duration::from_secs(1)).is_err());
    assert_eq!(fixture.count(), 1);
    fixture.display.entropy = entropy;
    fixture.display.poll(now + Duration::from_secs(2)).unwrap();
    assert_eq!(fixture.count(), 2);
}
#[test]
fn t920_retry_bounds_and_close_before_start() {
    for retry in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        let fixture = Fixture::new();
        assert!(Display::new(
            peer(1),
            fixture.attachment.clone(),
            Box::new(Backend(fixture.calls.clone())),
            entropy,
            retry
        )
        .is_err());
    }
    let mut fixture = Fixture::new();
    fixture.display.close();
    fixture.display.close();
    fixture.display.poll(Instant::now()).unwrap();
    assert_eq!(fixture.count(), 0);
    assert_eq!(fixture.display.state(), State::Retired);
}
#[test]
fn t920_replacement_during_retry_is_not_overwritten() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    let client = fixture.active(now);
    client.publish(Phase::Retired, Some(route(1)));
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now).unwrap();
    assert_eq!(fixture.display.state(), State::Waiting);
    let replacement = fixture
        .attachment
        .register_route(peer(2), Credential::new([44; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let lease = fixture.attachment.lease();
    let _ = fixture.display.poll(now + Duration::from_secs(2));
    assert_eq!(fixture.count(), 1, "T920 retry replaced foreign generation");
    drop(fixture.display);
    assert!(lease.apply(|| ()));
}
#[test]
fn t920_invalid_active_route_counters_never_admit_display() {
    for value in [0, i64::MAX as u64 + 1, u64::MAX] {
        for id in [
            RouteId {
                owner: value,
                revision: 1,
            },
            RouteId {
                owner: 1,
                revision: value,
            },
        ] {
            let mut fixture = Fixture::new();
            let now = Instant::now();
            fixture.display.poll(now).unwrap();
            fixture.client().publish(Phase::Active, Some(id));
            fixture.display.poll(now).unwrap();
            assert_eq!(
                fixture.display.state(),
                State::Retiring,
                "T920 invalid route admitted"
            );
            assert!(!*fixture.attachment.subscribe().borrow());
        }
    }
}
#[test]
fn t920_invalid_status_matrix_closes_without_admission() {
    use blent_config::connection::registration_client::Failure;
    let mut cases = vec![
        status(Phase::Starting, Some(route(1))),
        status(Phase::Registering, Some(route(1))),
        status(Phase::Queued, None),
        status(Phase::Active, None),
        status(Phase::Failed, None),
        status(Phase::Closing, None),
        status(Phase::Retired, None),
    ];
    for phase in [
        Phase::Starting,
        Phase::Registering,
        Phase::Queued,
        Phase::Active,
    ] {
        let id = if matches!(phase, Phase::Queued | Phase::Active) {
            Some(route(1))
        } else {
            None
        };
        let mut failed = status(phase, id);
        failed.failure = Some(Failure::Protocol);
        cases.push(failed);
        let mut retired = status(phase, id);
        retired.remote_retired = true;
        cases.push(retired);
    }
    for invalid in cases {
        let mut fixture = Fixture::new();
        let now = Instant::now();
        fixture.display.poll(now).unwrap();
        *fixture.client().0.status.lock().unwrap() = invalid;
        fixture.display.poll(now).unwrap();
        assert_eq!(fixture.display.state(), State::Retiring);
        assert!(fixture.client().0.closed.load(Ordering::SeqCst));
        assert!(!*fixture.attachment.subscribe().borrow());
    }
}
#[test]
fn t920_active_cannot_regress_or_change_route_and_repeat_poll_does_not_wake_capture() {
    for phase in [
        Phase::Starting,
        Phase::Registering,
        Phase::Queued,
        Phase::Active,
    ] {
        let mut fixture = Fixture::new();
        let now = Instant::now();
        let client = fixture.active(now);
        let mut presence = fixture.attachment.subscribe();
        presence.borrow_and_update();
        fixture.display.poll(now).unwrap();
        assert!(!presence.has_changed().unwrap());
        let id = if phase == Phase::Active {
            Some(route(2))
        } else {
            Some(route(1))
        };
        client.publish(phase, id);
        fixture.display.poll(now).unwrap();
        assert_eq!(fixture.display.state(), State::Retiring);
    }
}
#[test]
fn t920_idle_close_clears_only_owned_configuration() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    let client = fixture.active(now);
    fixture
        .settings
        .send_modify(|settings| settings.geometry_ready = true);
    client.publish(Phase::Retired, Some(route(1)));
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now).unwrap();
    assert_eq!(fixture.display.state(), State::Waiting);
    fixture.display.close();
    assert!(!*fixture.attachment.subscribe().borrow());
    assert!(!fixture.settings.borrow().geometry_ready);
    assert_eq!(fixture.display.state(), State::Retired);
    let next = fixture
        .attachment
        .register_route(peer(2), Credential::new([45; 32]), Link::Usb)
        .unwrap();
    assert!(next.admitted());
    let lease = fixture.attachment.lease();
    drop(fixture.display);
    assert!(lease.apply(|| ()));
}
#[test]
fn t920_first_start_cannot_replace_a_newer_owner() {
    let mut fixture = Fixture::new();
    let next = fixture
        .attachment
        .register_route(peer(2), Credential::new([46; 32]), Link::Usb)
        .unwrap();
    assert!(next.admitted());
    assert!(fixture.display.poll(Instant::now()).is_err());
    assert_eq!(fixture.count(), 0);
    assert_eq!(fixture.display.state(), State::Retired);
    assert!(next.current());
}
#[test]
fn t920_deadline_boundary_fails_without_registering() {
    let mut fixture = Fixture::new();
    let base = Instant::now();
    let mut seconds = 0;
    for bit in (0..64).rev() {
        let candidate = seconds | (1u64 << bit);
        if base.checked_add(Duration::from_secs(candidate)).is_some() {
            seconds = candidate;
        }
    }
    let far = base.checked_add(Duration::from_secs(seconds)).unwrap();
    assert!(fixture.display.poll(far).is_err());
    assert_eq!(fixture.count(), 0);
    assert_eq!(fixture.display.state(), State::Waiting);
}

#[test]
fn t1049_pause_retains_configuration_and_waits_for_original_registration() {
    let now = Instant::now();
    let mut fixture = Fixture::new();
    let client = fixture.active(now);
    fixture.settings.send_modify(|settings| {
        settings.width = 1920;
        settings.geometry_ready = true;
    });
    let original = fixture.attachment.lease();
    fixture.display.pause();
    assert_eq!(fixture.display.state(), State::Retiring);
    assert!(client.0.closed.load(Ordering::SeqCst));
    assert!(!original.apply(|| ()));
    assert!(*fixture.attachment.subscribe().borrow());
    fixture.display.resume();
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(
        fixture.count(),
        1,
        "T1049 early resume renewed pending owner"
    );
    client.publish(Phase::Retired, Some(route(1)));
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(fixture.display.state(), State::Waiting);
    fixture
        .display
        .poll(now + Duration::from_millis(60999))
        .unwrap();
    assert_eq!(fixture.count(), 1);
    fixture.display.poll(now + Duration::from_secs(61)).unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.requests.len(), 2);
    assert!(calls.requests[0].credential != calls.requests[1].credential);
    assert_eq!(fixture.settings.borrow().width, 1920);
    assert!(fixture.settings.borrow().geometry_ready);
}

#[test]
fn t1049_idle_pause_and_terminal_close_never_start_registration() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    fixture.display.pause();
    fixture.display.pause();
    for seconds in [0, 1, 60, 3600] {
        fixture
            .display
            .poll(now + Duration::from_secs(seconds))
            .unwrap();
        assert_eq!(fixture.display.state(), State::Paused);
    }
    assert_eq!(fixture.count(), 0);
    fixture.display.close();
    fixture.display.resume();
    fixture
        .display
        .poll(now + Duration::from_secs(7200))
        .unwrap();
    assert_eq!(fixture.count(), 0);
    assert_eq!(fixture.display.state(), State::Retired);
}

#[test]
fn t1049_late_active_reply_cannot_resume_a_paused_startup() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    fixture.display.poll(now).unwrap();
    let client = fixture.client();
    fixture.display.pause();
    client.publish(Phase::Active, Some(route(1)));
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(fixture.display.state(), State::Retiring);
    assert!(!*fixture.attachment.subscribe().borrow());
    client.publish(Phase::Retired, Some(route(1)));
    client.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(fixture.display.state(), State::Paused);
    fixture
        .display
        .poll(now + Duration::from_secs(120))
        .unwrap();
    assert_eq!(fixture.count(), 1);
    fixture.display.resume();
    fixture
        .display
        .poll(now + Duration::from_secs(120))
        .unwrap();
    assert_eq!(fixture.count(), 2);
}

#[test]
fn t1049_pause_close_and_resume_do_not_revoke_a_replacement() {
    let mut fixture = Fixture::new();
    let now = Instant::now();
    let original = fixture.active(now);
    fixture.display.pause();
    let replacement = fixture
        .attachment
        .register_route(peer(2), Credential::new([47; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    fixture.display.pause();
    original.publish(Phase::Retired, Some(route(1)));
    original.0.retired.store(true, Ordering::SeqCst);
    fixture.display.poll(now + Duration::from_secs(60)).unwrap();
    assert_eq!(fixture.display.state(), State::Paused);
    fixture.display.resume();
    assert!(fixture.display.poll(now + Duration::from_secs(61)).is_err());
    fixture.display.close();
    assert_eq!(fixture.count(), 1);
    assert!(
        replacement.current(),
        "T1049 stale pause touched replacement"
    );
}

fn transition(fixture: &mut Fixture, action: u32) {
    match action {
        0 => fixture.display.pause(),
        1 => fixture.display.resume(),
        2 => {
            if let Some(client) = fixture.calls.lock().unwrap().clients.last() {
                client.publish(Phase::Retired, Some(route(1)));
                client.0.retired.store(true, Ordering::SeqCst);
            }
        }
        _ => fixture.display.close(),
    }
}

#[test]
fn t1049_bounded_transitions_preserve_retirement_before_renewal() {
    let now = Instant::now();
    for pattern in 0..1024u32 {
        let mut fixture = Fixture::new();
        fixture.display.poll(now).unwrap();
        for step in 0..5 {
            transition(&mut fixture, (pattern >> (step * 2)) & 3);
            fixture
                .display
                .poll(now + Duration::from_secs(step + 1))
                .unwrap();
            let calls = fixture.calls.lock().unwrap();
            for client in calls.clients.iter().take(calls.clients.len() - 1) {
                assert!(
                    client.0.retired.load(Ordering::SeqCst),
                    "T1049 reused held owner"
                );
            }
        }
        fixture.display.close();
        for client in &fixture.calls.lock().unwrap().clients {
            client.publish(Phase::Retired, Some(route(1)));
            client.0.retired.store(true, Ordering::SeqCst);
        }
        fixture.display.poll(now + Duration::from_secs(10)).unwrap();
        assert_eq!(fixture.display.state(), State::Retired);
    }
}
