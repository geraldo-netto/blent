use super::*;
use crate::display_slots::{Owner as SlotOwner, Slots};
use crate::{
    display_discovery::{Discovery, Factory},
    media::EncoderSettings,
};
use blent_config::connection::{
    control::{Notice, State as LinkState},
    control_client::{
        Client as ControlClient, Phase as ControlPhase, Status as ControlStatus, SubmitError,
    },
    control_view::{Action, Assembler, Limits, Model, View},
    management::Credential,
    pairing::{PeerId, Ticket as PairingTicket},
    registration_client::{Client, Phase, Request as RegistrationRequest, Status},
    routes::{Link, RouteId},
};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tokio::sync::watch;

fn peer(id: u64) -> PeerId {
    PeerId::parse(format!("{id:064x}")).unwrap()
}
struct Control(View);
impl ControlClient for Control {
    fn status(&self) -> ControlStatus {
        ControlStatus {
            phase: ControlPhase::Ready,
            view: Some(self.0.clone()),
            message: None,
            outcome: None,
        }
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {}
    fn retired(&self) -> bool {
        false
    }
}
impl Factory for Control {
    fn start(&mut self) -> Result<Box<dyn ControlClient>> {
        Ok(Box::new(Self(self.0.clone())))
    }
}
fn candidate() -> Candidate {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 1,
            local: peer(1),
        })
        .unwrap();
    assembler
        .push(Notice::Link {
            revision: 1,
            connection: 1,
            ticket: Some(PairingTicket {
                epoch: 1,
                peer: peer(2),
            }),
            state: LinkState::Authenticated,
            trusted: false,
        })
        .unwrap();
    let view = Model::default()
        .publish(
            assembler
                .push(Notice::End { revision: 1 })
                .unwrap()
                .unwrap(),
        )
        .unwrap();
    let now = Instant::now();
    Discovery::new(Box::new(Control(view)), Duration::from_secs(1), now)
        .unwrap()
        .poll(now)
        .unwrap()
        .remove(0)
}
#[derive(Default)]
struct Native {
    failed: AtomicBool,
    done: AtomicBool,
    closes: AtomicUsize,
}
impl Work for Arc<Native> {
    fn alive(&self) -> bool {
        !self.failed.load(Ordering::SeqCst)
    }
    fn close(&mut self) {
        self.closes.fetch_add(1, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}
struct Route {
    status: Mutex<Status>,
    done: AtomicBool,
    closed: AtomicBool,
}
impl Route {
    fn set(&self, phase: Phase) {
        *self.status.lock().unwrap() = Status {
            phase,
            route: Some(RouteId {
                owner: 1,
                revision: 1,
            }),
            remote_retired: phase == Phase::Retired,
            failure: None,
        };
    }
}
struct Stub(Arc<Route>);
impl Client for Stub {
    fn status(&self) -> Status {
        self.0.status.lock().unwrap().clone()
    }
    fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.0.done.load(Ordering::SeqCst)
    }
}
#[derive(Default)]
struct Calls {
    requests: Vec<RegistrationRequest>,
    clients: Vec<Arc<Route>>,
    fail: bool,
}
struct Backend(Arc<Mutex<Calls>>);
impl Registration for Backend {
    fn start(&mut self, request: RegistrationRequest) -> Result<Box<dyn Client>> {
        let mut calls = self.0.lock().unwrap();
        calls.requests.push(request);
        ensure!(!calls.fail, "T925 unpublished registration failure");
        let client = Arc::new(Route {
            status: Mutex::new(Status {
                phase: Phase::Starting,
                route: None,
                remote_retired: false,
                failure: None,
            }),
            done: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        });
        calls.clients.push(client.clone());
        Ok(Box::new(Stub(client)))
    }
}
fn entropy(bytes: &mut [u8]) -> Result<()> {
    static NEXT: AtomicU8 = AtomicU8::new(1);
    bytes.fill(NEXT.fetch_add(1, Ordering::SeqCst));
    Ok(())
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
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}
struct Fixture {
    session: Session,
    slots: Slots,
    native: Arc<Native>,
    calls: Arc<Mutex<Calls>>,
    attachment: Attachment,
    settings: watch::Sender<EncoderSettings>,
}
impl Fixture {
    fn new() -> Self {
        let candidate = candidate();
        let mut slots = Slots::new(1).unwrap();
        slots.select(Some(candidate.peer().clone()));
        let ticket = slots
            .reserve(SlotOwner::Peer(candidate.peer().clone()), 0)
            .unwrap();
        let request = Request::new(
            &slots,
            candidate,
            ticket,
            (8890, 8891),
            Duration::from_secs(1),
        )
        .unwrap();
        let native = Arc::new(Native::default());
        let calls = Arc::new(Mutex::new(Calls::default()));
        let (settings, _) = watch::channel(settings());
        let attachment = Attachment::new(settings.clone());
        let session = request.bind(
            &slots,
            Bindings {
                attachment: attachment.clone(),
                registration: Box::new(Backend(calls.clone())),
                entropy,
                work: Box::new(native.clone()),
            },
        );
        Self {
            session,
            slots,
            native,
            calls,
            attachment,
            settings,
        }
    }
    fn client(&self) -> Arc<Route> {
        self.calls.lock().unwrap().clients.last().unwrap().clone()
    }
    fn active(&mut self, now: Instant) -> Arc<Route> {
        assert_eq!(self.session.state(), State::Waiting);
        assert!(self.session.status().is_none());
        self.session.poll(now).unwrap();
        assert_eq!(self.session.state(), State::Registering);
        let client = self.client();
        client.set(Phase::Active);
        self.session.poll(now).unwrap();
        assert_eq!(self.session.state(), State::Active);
        client
    }
}
#[test]
fn t925_capacity_waits_for_both_registration_and_native_session() {
    for native_first in [false, true] {
        let now = Instant::now();
        let mut fixture = Fixture::new();
        let client = fixture.active(now);
        let status = fixture.session.status().unwrap();
        assert_eq!(status.identity.peer(), Some(&peer(2)));
        assert!(status.identity.adb_serial().is_none());
        assert_eq!(
            (status.instance, status.video_port, status.input_port),
            (0, 8890, 8891)
        );
        assert!(fixture
            .session
            .candidate()
            .same_connection(fixture.session.candidate()));
        let ticket = fixture.session.ticket();
        fixture.slots.close(ticket).unwrap();
        fixture.session.close();
        fixture.session.close();
        assert!(fixture.session.closing());
        assert!(client.closed.load(Ordering::SeqCst));
        assert_eq!(fixture.native.closes.load(Ordering::SeqCst), 1);
        assert!(!fixture.session.retired());
        assert!(fixture.session.status().is_none());
        if native_first {
            fixture.native.done.store(true, Ordering::SeqCst);
        } else {
            client.done.store(true, Ordering::SeqCst);
        }
        fixture.session.poll(now).unwrap();
        assert_eq!(fixture.session.state(), State::Retiring);
        assert!(!fixture.session.retired());
        assert!(fixture.slots.reserve(SlotOwner::Peer(peer(2)), 0).is_err());
        fixture.native.done.store(true, Ordering::SeqCst);
        client.done.store(true, Ordering::SeqCst);
        fixture.session.poll(now).unwrap();
        assert!(fixture.session.retired());
        assert_eq!(fixture.session.state(), State::Retired);
        fixture.slots.retired(ticket).unwrap();
        assert!(fixture.slots.reserve(SlotOwner::Peer(peer(2)), 0).is_ok());
    }
}
#[test]
fn t925_registration_renewal_preserves_runtime_geometry_and_fresh_credentials() {
    let now = Instant::now();
    let mut fixture = Fixture::new();
    let client = fixture.active(now);
    fixture.settings.send_modify(|settings| {
        settings.width = 1920;
        settings.geometry_ready = true;
    });
    client.set(Phase::Retired);
    fixture.session.poll(now).unwrap();
    assert_eq!(fixture.session.state(), State::Retiring);
    assert_eq!(fixture.native.closes.load(Ordering::SeqCst), 0);
    assert!(*fixture.attachment.subscribe().borrow());
    client.done.store(true, Ordering::SeqCst);
    fixture.session.poll(now).unwrap();
    fixture.session.poll(now + Duration::from_secs(1)).unwrap();
    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.requests.len(), 2);
    assert!(calls.requests[0].credential != calls.requests[1].credential);
    drop(calls);
    assert_eq!(fixture.settings.borrow().width, 1920);
    assert!(fixture.settings.borrow().geometry_ready);
    assert_eq!(fixture.native.closes.load(Ordering::SeqCst), 0);
}
#[test]
fn t925_native_failure_closes_only_its_own_registration() {
    let now = Instant::now();
    let mut failed = Fixture::new();
    let mut sibling = Fixture::new();
    let client = failed.active(now);
    let other = sibling.active(now);
    failed.native.failed.store(true, Ordering::SeqCst);
    assert!(failed.session.status().is_none());
    failed.session.poll(now).unwrap();
    assert!(failed.session.closing());
    assert!(client.closed.load(Ordering::SeqCst));
    assert_eq!(sibling.session.state(), State::Active);
    assert!(!other.closed.load(Ordering::SeqCst));
    assert_eq!(sibling.native.closes.load(Ordering::SeqCst), 0);
}
#[test]
fn t925_stale_authority_and_late_cancel_cannot_revoke_a_replacement() {
    let now = Instant::now();
    let mut fixture = Fixture::new();
    let client = fixture.active(now);
    let replacement = fixture
        .attachment
        .register_route(peer(3), Credential::new([42; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let lease = fixture.attachment.lease();
    fixture.session.poll(now).unwrap();
    client.done.store(true, Ordering::SeqCst);
    fixture.session.poll(now).unwrap();
    assert!(fixture.session.closing());
    assert_eq!(fixture.native.closes.load(Ordering::SeqCst), 1);
    assert!(lease.apply(|| ()));
    fixture.session.close();
    assert!(lease.apply(|| ()));
}
#[test]
fn t925_drop_and_unpublished_start_failure_keep_cleanup_owned() {
    let now = Instant::now();
    let mut fixture = Fixture::new();
    fixture.calls.lock().unwrap().fail = true;
    assert!(fixture.session.poll(now).is_err());
    assert_eq!(fixture.session.state(), State::Waiting);
    assert!(!fixture.session.retired());
    let native = fixture.native.clone();
    drop(fixture);
    assert_eq!(native.closes.load(Ordering::SeqCst), 1);
    let mut fixture = Fixture::new();
    let client = fixture.active(now);
    let native = fixture.native.clone();
    drop(fixture);
    assert!(client.closed.load(Ordering::SeqCst));
    assert_eq!(native.closes.load(Ordering::SeqCst), 1);
}
#[test]
fn t925_preflight_rejects_wrong_stale_capacity_ports_and_retry_bounds() {
    let candidate = candidate();
    let mut slots = Slots::new(1).unwrap();
    slots.select(Some(peer(2)));
    let ticket = slots.reserve(SlotOwner::Peer(peer(2)), 0).unwrap();
    for ports in [(0, 1), (1, 0), (1, 1), (u16::MAX, u16::MAX)] {
        assert!(Request::new(
            &slots,
            candidate.clone(),
            ticket,
            ports,
            Duration::from_secs(1)
        )
        .is_err());
    }
    for retry in [
        Duration::ZERO,
        Duration::from_secs(60) + Duration::from_nanos(1),
        Duration::MAX,
    ] {
        assert!(Request::new(&slots, candidate.clone(), ticket, (1, u16::MAX), retry).is_err());
    }
    for retry in [Duration::from_nanos(1), Duration::from_secs(60)] {
        assert!(Request::new(&slots, candidate.clone(), ticket, (u16::MAX, 1), retry).is_ok());
    }
    slots.close(ticket).unwrap();
    slots.retired(ticket).unwrap();
    assert!(Request::new(
        &slots,
        candidate.clone(),
        ticket,
        (1, 2),
        Duration::from_secs(1)
    )
    .is_err());
    slots.select(Some(peer(3)));
    let other = slots.reserve(SlotOwner::Peer(peer(3)), 0).unwrap();
    assert!(Request::new(&slots, candidate, other, (1, 2), Duration::from_secs(1)).is_err());
}

#[test]
fn t925_closing_capacity_cannot_start_another_display_owner() {
    let candidate = candidate();
    let mut slots = Slots::new(1).unwrap();
    slots.select(Some(peer(2)));
    let ticket = slots.reserve(SlotOwner::Peer(peer(2)), 0).unwrap();
    slots.close(ticket).unwrap();
    assert!(
        Request::new(
            &slots,
            candidate,
            ticket,
            (8890, 8891),
            Duration::from_secs(1)
        )
        .is_err(),
        "T925 closing capacity admitted another startup"
    );
}

#[test]
fn t925_cancel_before_admission_and_late_active_cannot_revive_session() {
    let now = Instant::now();
    let mut fixture = Fixture::new();
    fixture.session.poll(now).unwrap();
    let client = fixture.client();
    fixture.session.close();
    client.set(Phase::Active);
    fixture.session.poll(now).unwrap();
    assert!(!*fixture.attachment.subscribe().borrow());
    assert_eq!(fixture.session.state(), State::Retiring);
    assert!(fixture.session.status().is_none());
    client.done.store(true, Ordering::SeqCst);
    fixture.native.done.store(true, Ordering::SeqCst);
    fixture.session.poll(now).unwrap();
    assert!(fixture.session.retired());
    let mut unpublished = Fixture::new();
    unpublished.session.close();
    unpublished.session.poll(now).unwrap();
    assert!(unpublished.calls.lock().unwrap().requests.is_empty());
    assert!(!unpublished.session.retired());
    unpublished.native.done.store(true, Ordering::SeqCst);
    assert!(unpublished.session.retired());
}

#[test]
fn t925_cancelled_preflight_cannot_bind_over_replacement_authority() {
    let candidate = candidate();
    let mut slots = Slots::new(1).unwrap();
    slots.select(Some(peer(2)));
    let ticket = slots.reserve(SlotOwner::Peer(peer(2)), 0).unwrap();
    let request = Request::new(
        &slots,
        candidate,
        ticket,
        (8890, 8891),
        Duration::from_secs(1),
    )
    .unwrap();
    slots.close(ticket).unwrap();
    slots.retired(ticket).unwrap();
    slots.select(Some(peer(3)));
    slots.reserve(SlotOwner::Peer(peer(3)), 0).unwrap();
    let (settings, _) = watch::channel(settings());
    let attachment = Attachment::new(settings);
    let replacement = attachment
        .register_route(peer(3), Credential::new([42; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let lease = attachment.lease();
    let native = Arc::new(Native::default());
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut session = request.bind(
        &slots,
        Bindings {
            attachment: attachment.clone(),
            registration: Box::new(Backend(calls.clone())),
            entropy,
            work: Box::new(native.clone()),
        },
    );
    assert!(
        session.closing(),
        "T925 cancelled preflight created another display owner"
    );
    session.poll(Instant::now()).unwrap();
    assert!(calls.lock().unwrap().requests.is_empty());
    assert!(
        lease.apply(|| ()),
        "T925 late bind revoked replacement authority"
    );
    assert!(!session.retired());
    native.done.store(true, Ordering::SeqCst);
    assert!(session.retired());
}
