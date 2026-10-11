use super::*;
use crate::{attachment::Attachment, display_test_support::*};
use blent_config::connection::{
    control::{Notice, State},
    control_client::{Client, Phase as ControlPhase, Status as ControlStatus, SubmitError},
    control_view::{Action, Assembler, Limits, Model},
    pairing::Ticket as PairingTicket,
    registration_client::Phase,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::watch;

struct Source(Arc<Mutex<ControlStatus>>, Arc<AtomicBool>);
impl Client for Source {
    fn status(&self) -> ControlStatus {
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
impl blent_config::connection::control_client::Factory for Source {
    fn start(&mut self) -> Result<Box<dyn Client>> {
        Ok(Box::new(Self(self.0.clone(), self.1.clone())))
    }
}
struct Fixture {
    connections: Connections,
    slots: Slots,
    source: Arc<Mutex<ControlStatus>>,
    model: Model,
    revision: u64,
    now: Instant,
}
impl Fixture {
    fn new(capacity: u32) -> Self {
        let now = Instant::now();
        let source = Arc::new(Mutex::new(ControlStatus {
            phase: ControlPhase::Ready,
            view: None,
            message: None,
            outcome: None,
        }));
        let discovery = Discovery::new(
            Box::new(Source(source.clone(), Arc::new(AtomicBool::new(false)))),
            Duration::from_secs(1),
            now,
        )
        .unwrap();
        let ports = (0..capacity)
            .map(|slot| ((18000 + slot * 2) as u16, (18001 + slot * 2) as u16))
            .collect();
        Self {
            connections: Connections::new(discovery, ports, Duration::from_secs(1)).unwrap(),
            slots: Slots::new(capacity).unwrap(),
            source,
            model: Model::default(),
            revision: 0,
            now,
        }
    }
    fn links(&mut self, links: &[(u64, u64, u64)]) {
        self.revision += 1;
        let mut assembler = Assembler::new(Limits::default()).unwrap();
        assembler
            .push(Notice::Begin {
                revision: self.revision,
                local: peer(1),
            })
            .unwrap();
        for &(connection, identity, epoch) in links {
            assembler
                .push(Notice::Link {
                    revision: self.revision,
                    connection,
                    ticket: Some(PairingTicket {
                        epoch,
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
        self.poll();
    }
    fn poll(&mut self) {
        self.connections.poll(&mut self.slots, self.now).unwrap();
    }
    fn start(&mut self) -> Admission {
        match self.connections.plan(&mut self.slots, self.now).unwrap() {
            Decision::Start(admission) => admission,
            _ => panic!("T927 expected admission"),
        }
    }
    fn bind(&mut self, admission: Admission) -> (Arc<Native>, Arc<Route>) {
        let native = Arc::new(Native::default());
        let calls = Arc::new(Mutex::new(Calls::default()));
        let attachment = Attachment::new(watch::channel(settings()).0);
        self.connections.bind(
            &mut self.slots,
            admission,
            Bindings {
                attachment,
                registration: Box::new(Backend(calls.clone())),
                entropy,
                work: Box::new(native.clone()),
            },
        );
        self.poll();
        let client = calls.lock().unwrap().clients.last().unwrap().clone();
        client.set(Phase::Active);
        self.poll();
        (native, client)
    }
}
#[test]
fn t927_all_peers_progress_and_ambiguous_new_admission_never_preempts_adb() {
    let mut f = Fixture::new(3);
    let adb = f.slots.reserve(Owner::Adb("ADB".into()), 0).unwrap();
    f.links(&[(1, 2, 1), (2, 2, 1)]);
    assert!(matches!(
        f.connections.plan(&mut f.slots, f.now).unwrap(),
        Decision::None
    ));
    assert_eq!(f.slots.closing(adb), Some(false));
    f.links(&[(1, 2, 1), (3, 3, 1)]);
    let first = f.start();
    assert_eq!(first.instance(), 1);
    let _first = f.bind(first);
    let second = f.start();
    assert_eq!(second.instance(), 2);
    let _second = f.bind(second);
    assert_eq!(f.connections.status().len(), 2);
    assert!(matches!(
        f.connections.plan(&mut f.slots, f.now).unwrap(),
        Decision::None
    ));
    f.links(&[(1, 2, 1), (2, 2, 1), (3, 3, 1), (4, 4, 1)]);
    assert_eq!(
        f.connections.status().len(),
        2,
        "T927 duplicate invalidated existing exact link"
    );
    assert!(
        matches!(f.connections.plan(&mut f.slots,f.now).unwrap(),Decision::Replace(ticket) if ticket==adb)
    );
    f.slots.close(adb).unwrap();
    assert!(matches!(
        f.connections.plan(&mut f.slots, f.now).unwrap(),
        Decision::Wait
    ));
    f.slots.retired(adb).unwrap();
    let third = f.start();
    assert_eq!(third.instance(), 0);
    f.connections.failed(&mut f.slots, third, f.now);
}
#[test]
fn t927_source_loss_and_changed_ticket_hold_capacity_until_joined_cleanup() {
    for native_first in [false, true] {
        let mut f = Fixture::new(1);
        f.links(&[(1, 2, 1)]);
        let admission = f.start();
        let (native, route) = f.bind(admission);
        let ticket = f.slots.ticket(&Owner::Peer(peer(2))).unwrap();
        f.links(&[(1, 2, 2)]);
        assert!(f.connections.status().is_empty());
        assert_eq!(f.slots.closing(ticket), Some(true));
        assert!(matches!(
            f.connections.plan(&mut f.slots, f.now).unwrap(),
            Decision::None
        ));
        if native_first {
            native.done.store(true, Ordering::SeqCst);
        } else {
            route.done.store(true, Ordering::SeqCst);
            route.set(Phase::Retired);
        }
        f.poll();
        assert!(f.slots.owner(ticket).is_some());
        native.done.store(true, Ordering::SeqCst);
        route.done.store(true, Ordering::SeqCst);
        route.set(Phase::Retired);
        f.poll();
        assert!(f.slots.owner(ticket).is_none());
        f.now += Duration::from_secs(1);
        f.poll();
        let next = f.start();
        f.connections.failed(&mut f.slots, next, f.now);
    }
}
#[test]
fn t927_cancelled_start_is_retained_and_cannot_publish_late_authority() {
    let mut f = Fixture::new(1);
    f.links(&[(1, 2, 1)]);
    let admission = f.start();
    let ticket = admission.ticket;
    f.links(&[]);
    assert_eq!(f.slots.closing(ticket), Some(true));
    let native = Arc::new(Native::default());
    let calls = Arc::new(Mutex::new(Calls::default()));
    f.connections.bind(
        &mut f.slots,
        admission,
        Bindings {
            attachment: Attachment::new(watch::channel(settings()).0),
            registration: Box::new(Backend(calls.clone())),
            entropy,
            work: Box::new(native.clone()),
        },
    );
    f.poll();
    assert!(calls.lock().unwrap().requests.is_empty());
    assert!(f.slots.owner(ticket).is_some());
    native.done.store(true, Ordering::SeqCst);
    f.poll();
    assert!(f.slots.owner(ticket).is_none());
    f.connections.close(&mut f.slots);
    f.poll();
    assert!(f.connections.retired());
}
#[test]
fn t927_failed_start_has_absolute_retry_and_does_not_starve_other_peer() {
    let mut f = Fixture::new(1);
    f.links(&[(1, 2, 1)]);
    let admission = f.start();
    f.connections.failed(&mut f.slots, admission, f.now);
    for _ in 0..100 {
        f.poll();
        assert!(matches!(
            f.connections.plan(&mut f.slots, f.now).unwrap(),
            Decision::None
        ));
    }
    f.links(&[(1, 2, 1), (2, 3, 1)]);
    let other = f.start();
    assert_eq!(other.candidate.peer(), &peer(3));
    f.connections.failed(&mut f.slots, other, f.now);
    f.now += Duration::from_secs(1);
    f.poll();
    let next = f.start();
    assert_eq!(next.candidate.peer(), &peer(3));
    f.connections.close(&mut f.slots);
    assert!(!f.connections.retired());
    f.connections.failed(&mut f.slots, next, f.now);
    f.poll();
    assert!(f.connections.retired());
    assert!(matches!(
        f.connections.plan(&mut f.slots, f.now).unwrap(),
        Decision::None
    ));
}
#[test]
fn t927_external_capacity_cancel_retires_active_owner() {
    let mut f = Fixture::new(1);
    f.links(&[(1, 2, 1)]);
    let admission = f.start();
    let ticket = admission.ticket;
    let (_, route) = f.bind(admission);
    f.slots.close(ticket).unwrap();
    f.poll();
    assert!(
        f.connections.status().is_empty(),
        "T927 closing capacity retained active owner"
    );
    assert!(route.closed.load(Ordering::SeqCst));
}

fn discovery() -> Discovery {
    Discovery::new(
        Box::new(Source(
            Arc::new(Mutex::new(ControlStatus {
                phase: ControlPhase::Connecting,
                view: None,
                message: None,
                outcome: None,
            })),
            Arc::new(AtomicBool::new(false)),
        )),
        Duration::from_secs(1),
        Instant::now(),
    )
    .unwrap()
}
#[test]
fn t927_bounded_invalid_ports_capacity_and_retry_fail_before_native_work() {
    for count in 0..=8 {
        let ports = (0..count)
            .map(|slot| ((20000 + slot * 2) as u16, (20001 + slot * 2) as u16))
            .collect();
        assert_eq!(
            Connections::new(discovery(), ports, Duration::from_secs(1)).is_ok(),
            (1..=4).contains(&count)
        );
    }
    for value in [0, 1, 2, 65534, 65535] {
        for ports in [
            vec![(value, value)],
            vec![(value, 0)],
            vec![(0, value)],
            vec![(value, 65535), (value, 65534)],
        ] {
            assert!(Connections::new(discovery(), ports, Duration::from_secs(1)).is_err());
        }
    }
    for retry in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        assert!(Connections::new(discovery(), vec![(1, 65535)], retry).is_err());
    }
    for retry in [Duration::from_nanos(1), Duration::from_secs(60)] {
        assert!(Connections::new(discovery(), vec![(1, 65535)], retry).is_ok());
    }
    let mut f = Fixture::new(1);
    f.slots = Slots::new(2).unwrap();
    f.slots.reserve(Owner::Adb("ADB".into()), 0).unwrap();
    f.links(&[(1, 2, 1)]);
    assert!(f.connections.plan(&mut f.slots, f.now).is_err());
    assert!(f.slots.ticket(&Owner::Peer(peer(2))).is_none());
}
#[test]
fn t927_registration_failure_does_not_close_other_owner_or_hide_retirement() {
    let mut f = Fixture::new(2);
    f.links(&[(1, 2, 1), (2, 3, 1)]);
    let first = f.start();
    let (native, route) = f.bind(first);
    let second = f.start();
    let other_native = Arc::new(Native::default());
    let calls = Arc::new(Mutex::new(Calls {
        fail: true,
        ..Default::default()
    }));
    f.connections.bind(
        &mut f.slots,
        second,
        Bindings {
            attachment: Attachment::new(watch::channel(settings()).0),
            registration: Box::new(Backend(calls.clone())),
            entropy,
            work: Box::new(other_native.clone()),
        },
    );
    assert!(f.connections.poll(&mut f.slots, f.now).is_err());
    assert_eq!(f.connections.status().len(), 1);
    assert!(!route.closed.load(Ordering::SeqCst));
    assert_eq!(calls.lock().unwrap().requests.len(), 1);
    f.poll();
    assert_eq!(calls.lock().unwrap().requests.len(), 1);
    native.failed.store(true, Ordering::SeqCst);
    f.poll();
    assert!(route.closed.load(Ordering::SeqCst));
    native.done.store(true, Ordering::SeqCst);
    route.set(Phase::Retired);
    route.done.store(true, Ordering::SeqCst);
    f.connections.close(&mut f.slots);
    f.poll();
    assert!(!f.connections.retired());
    other_native.done.store(true, Ordering::SeqCst);
    f.poll();
    assert!(f.connections.retired());
}
struct Failed;
impl blent_config::connection::control_client::Factory for Failed {
    fn start(&mut self) -> Result<Box<dyn Client>> {
        anyhow::bail!("T927 discovery unavailable")
    }
}
#[test]
fn t927_discovery_failure_still_processes_retained_cleanup() {
    let now = Instant::now();
    let source = Discovery::new(Box::new(Failed), Duration::from_secs(1), now).unwrap();
    let mut connections =
        Connections::new(source, vec![(18000, 18001)], Duration::from_secs(1)).unwrap();
    let mut slots = Slots::new(1).unwrap();
    assert!(connections.poll(&mut slots, now).is_err());
    assert!(matches!(
        connections.plan(&mut slots, now).unwrap(),
        Decision::None
    ));
    connections.close(&mut slots);
    connections.poll(&mut slots, now).unwrap();
    assert!(connections.retired());
}

fn latest_instant(now: Instant) -> Instant {
    let (mut low, mut high) = (0, u64::MAX);
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if now.checked_add(Duration::from_secs(middle)).is_some() {
            low = middle;
        } else {
            high = middle;
        }
    }
    now.checked_add(Duration::from_secs(low)).unwrap()
}
#[test]
fn t927_exhausted_retry_clock_cannot_spin_or_preempt_another_display() {
    let mut f = Fixture::new(2);
    let adb = f.slots.reserve(Owner::Adb("ADB".into()), 0).unwrap();
    f.links(&[(1, 2, 1)]);
    let admission = f.start();
    f.now = latest_instant(f.now);
    assert!(f.now.checked_add(Duration::from_secs(1)).is_none());
    f.connections.failed(&mut f.slots, admission, f.now);
    for _ in 0..100 {
        f.poll();
        assert!(matches!(
            f.connections.plan(&mut f.slots, f.now).unwrap(),
            Decision::None
        ));
    }
    assert_eq!(f.slots.closing(adb), Some(false));
}
