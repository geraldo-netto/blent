//! T1051: selected display registration joins original broker/native retirement.
use super::*;
use crate::display_quiescence::{channel, Participant};
use blent_config::host_activity::*;

struct ActivityFixture {
    host: Fixture,
    participants: Vec<Participant>,
    published: watch::Sender<Publication>,
    feed: Feed,
    original_runtime: Option<blent_config::retirement::Admission>,
}
impl ActivityFixture {
    fn new(pending: bool) -> Self {
        let capabilities = Capabilities {
            power: true,
            lock: true,
            display: true,
        };
        let mut feed = Feed::new(1, capabilities, Settings::default()).unwrap();
        feed.observe(Event {
            epoch: 1,
            revision: 1,
            state: HostState {
                power: Some(Power::Running),
                locked: Some(false),
                display_on: Some(true),
            },
        })
        .unwrap();
        let publication = if pending {
            Publication::Pending
        } else {
            Publication::Observed {
                capabilities,
                snapshot: feed.snapshot(true, false),
            }
        };
        let (published, source) = watch::channel(publication);
        let (control, participants) = channel(2).unwrap();
        let owner = blent_config::retirement::Pool::new(1)
            .unwrap()
            .reserve()
            .unwrap();
        let host = Fixture::with_activity(Some(ActivityBinding {
            control: control.clone(),
            retirement: owner.completion(),
            source: Some(source),
        }));
        Self {
            host,
            participants,
            published,
            feed,
            original_runtime: Some(owner),
        }
    }
    fn observe(&mut self, revision: u64, locked: bool) {
        self.feed
            .observe(Event {
                epoch: 1,
                revision,
                state: HostState {
                    power: Some(Power::Running),
                    locked: Some(locked),
                    display_on: Some(!locked),
                },
            })
            .unwrap();
        self.published.send_replace(Publication::Observed {
            capabilities: Capabilities {
                power: true,
                lock: true,
                display: true,
            },
            snapshot: self.feed.snapshot(true, false),
        });
    }
    fn count(&self) -> usize {
        self.host.calls.lock().unwrap().requests.len()
    }
    fn acknowledge(&self) {
        for participant in &self.participants {
            if let Some(pause) = participant.pending() {
                assert!(participant.acknowledge(&pause));
            }
        }
    }
    fn poll(&mut self, now: Instant) {
        self.host.session.poll(now).unwrap();
    }
    fn active(&mut self, now: Instant) -> Arc<Route> {
        assert_eq!(self.count(), 0);
        self.acknowledge();
        self.poll(now);
        self.poll(now);
        assert_eq!(self.count(), 1);
        let client = self.host.client();
        client.set(Phase::Active);
        self.poll(now);
        assert_eq!(self.host.session.state(), State::Active);
        assert!(self.participants[0].source().begin().is_some());
        client
    }
}

#[test]
fn t1051_pending_activity_cannot_register_or_release_selected_capacity() {
    let mut fixture = ActivityFixture::new(true);
    let ticket = fixture.host.session.ticket();
    fixture.poll(Instant::now());
    assert_eq!(
        fixture.count(),
        0,
        "T1051 pending activity started a broker route"
    );
    assert_eq!(fixture.host.session.state(), State::Paused);
    assert!(!fixture.host.session.closing());
    assert!(fixture
        .host
        .slots
        .open_peer(ticket, fixture.host.session.candidate().peer()));
    assert_eq!(fixture.host.native.closes.load(Ordering::SeqCst), 0);
    assert!(fixture.participants[0].source().begin().is_none());
}

#[test]
fn t1051_resume_waits_for_broker_and_every_original_native_receipt() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    let old = fixture.active(now);
    fixture.host.settings.send_modify(|settings| {
        settings.width = 1920;
        settings.geometry_ready = true;
    });
    let lease = fixture.host.attachment.lease();
    let ticket = fixture.host.session.ticket();
    fixture.observe(2, true);
    fixture.poll(now);
    assert!(old.closed.load(Ordering::SeqCst));
    assert!(!lease.apply(|| ()));
    assert_eq!(fixture.host.native.closes.load(Ordering::SeqCst), 0);
    fixture.observe(3, false);
    fixture.poll(now + Duration::from_secs(10));
    fixture.acknowledge();
    fixture.poll(now + Duration::from_secs(20));
    assert_eq!(
        fixture.count(),
        1,
        "T1051 native receipts substituted for broker retirement"
    );
    old.set(Phase::Retired);
    old.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(20));
    fixture.poll(now + Duration::from_secs(21));
    assert_eq!(fixture.count(), 2);
    assert!(
        fixture.participants[0].source().begin().is_none(),
        "T1051 native work resumed before new route Active"
    );
    fixture.host.client().set(Phase::Active);
    fixture.poll(now + Duration::from_secs(21));
    assert!(fixture.participants[0].source().begin().is_some());
    assert_eq!(fixture.host.session.ticket(), ticket);
    assert_eq!(fixture.host.settings.borrow().width, 1920);
    assert!(fixture.host.settings.borrow().geometry_ready);
    let calls = fixture.host.calls.lock().unwrap();
    assert!(calls.requests[0].credential != calls.requests[1].credential);
}

#[test]
fn t1051_broker_retirement_cannot_release_a_held_native_participant() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    let old = fixture.active(now);
    fixture.observe(2, true);
    fixture.poll(now);
    fixture.observe(3, false);
    old.set(Phase::Retired);
    old.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(10));
    let participant = &fixture.participants[0];
    participant.acknowledge(&participant.pending().unwrap());
    fixture.poll(now + Duration::from_secs(60));
    assert_eq!(
        fixture.count(),
        1,
        "T1051 original selector/capture still owned cleanup"
    );
    let participant = &fixture.participants[1];
    participant.acknowledge(&participant.pending().unwrap());
    fixture.poll(now + Duration::from_secs(60));
    fixture.poll(now + Duration::from_secs(61));
    assert_eq!(fixture.count(), 2);
}

#[test]
fn t1051_coalesced_stop_cancels_late_startup_and_renewal_without_reconnect_storm() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    fixture.acknowledge();
    fixture.poll(now);
    fixture.poll(now);
    let original = fixture.host.client();
    fixture.observe(2, true);
    fixture.observe(3, false);
    original.set(Phase::Active);
    fixture.poll(now);
    assert!(original.closed.load(Ordering::SeqCst));
    assert!(fixture.host.session.status().is_none());
    assert!(fixture.participants[0].source().begin().is_none());
    for seconds in 1..128 {
        fixture.poll(now + Duration::from_secs(seconds));
    }
    assert_eq!(fixture.count(), 1);
    fixture.acknowledge();
    original.set(Phase::Retired);
    original.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(128));
    fixture.poll(now + Duration::from_secs(129));
    assert_eq!(fixture.count(), 2);
}

#[test]
fn t1051_foreign_replacement_is_not_revoked_or_paused_by_late_owner_close() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    let original = fixture.active(now);
    let replacement = fixture
        .host
        .attachment
        .register_route(peer(3), Credential::new([93; 32]), Link::Usb)
        .unwrap();
    assert!(replacement.admitted());
    let source = fixture.participants[0].source();
    let permit = source.begin().unwrap();
    fixture.host.session.close();
    original.set(Phase::Retired);
    original.done.store(true, Ordering::SeqCst);
    fixture.host.native.done.store(true, Ordering::SeqCst);
    fixture.poll(now);
    assert!(replacement.current());
    assert!(
        source.permits(&permit),
        "T1051 stale owner paused replacement native work"
    );
    assert!(
        !fixture.host.session.retired(),
        "T1051 unknown original native obligations were released"
    );
    drop(fixture.original_runtime.take());
    assert!(fixture.host.session.retired());
    assert!(replacement.current());
}

#[test]
fn t1051_unavailable_sources_pause_without_closing_unrelated_work_or_releasing_capacity() {
    for publication in [
        Publication::Pending,
        Publication::Failed,
        Publication::Stopped,
    ] {
        let now = Instant::now();
        let mut fixture = ActivityFixture::new(false);
        let original = fixture.active(now);
        fixture.published.send_replace(publication);
        fixture.poll(now);
        assert!(original.closed.load(Ordering::SeqCst));
        assert_eq!(fixture.host.native.closes.load(Ordering::SeqCst), 0);
        assert!(!fixture.host.session.closing());
        fixture.acknowledge();
        original.set(Phase::Retired);
        original.done.store(true, Ordering::SeqCst);
        fixture.poll(now + Duration::from_secs(1));
        fixture.poll(now + Duration::from_secs(60));
        assert_eq!(fixture.count(), 1);
        assert_eq!(fixture.host.session.state(), State::Paused);
    }
}

#[test]
fn t1051_closed_observer_or_dead_native_producer_cannot_renew() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    let old = fixture.active(now);
    drop(fixture.published);
    fixture.host.session.poll(now).unwrap();
    assert!(old.closed.load(Ordering::SeqCst));
    assert!(fixture.participants[0].source().begin().is_none());
    let mut fixture = ActivityFixture::new(false);
    let old = fixture.active(now);
    fixture.observe(2, true);
    fixture.poll(now);
    fixture.observe(3, false);
    fixture.participants.pop();
    fixture.acknowledge();
    old.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(1));
    fixture.poll(now + Duration::from_secs(60));
    assert_eq!(fixture.count(), 1);
    fixture.host.session.close();
    fixture.host.native.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(60));
    assert!(!fixture.host.session.retired());
    drop(fixture.original_runtime.take());
    assert!(fixture.host.session.retired());
}

#[test]
fn t1051_registration_failure_retires_before_retry_and_gates_native_work() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    fixture.acknowledge();
    fixture.host.calls.lock().unwrap().fail = true;
    assert!(fixture.host.session.poll(now).is_err());
    assert!(fixture.participants[0].source().begin().is_none());
    fixture.host.calls.lock().unwrap().fail = false;
    fixture.poll(now + Duration::from_secs(1));
    let route = fixture.host.client();
    route.set(Phase::Active);
    fixture.poll(now + Duration::from_secs(1));
    assert!(fixture.participants[0].source().begin().is_some());
    route.set(Phase::Retired);
    fixture.poll(now + Duration::from_secs(2));
    assert!(fixture.participants[0].source().begin().is_none());
    fixture.acknowledge();
    fixture.poll(now + Duration::from_secs(60));
    assert_eq!(fixture.count(), 2);
    route.done.store(true, Ordering::SeqCst);
    fixture.poll(now + Duration::from_secs(60));
    fixture.poll(now + Duration::from_secs(61));
    assert_eq!(fixture.count(), 3);
}
