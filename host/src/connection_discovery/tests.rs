use super::*;
use blent_config::connection::{
    control::{Command, Notice},
    control_client::{Outcome, SubmitError},
    control_view::{Action, Assembler, Limits, Model},
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

fn peer(id: u64) -> PeerId {
    PeerId::parse(format!("{id:064x}")).unwrap()
}
fn snapshot(model: &mut Model, revision: u64, links: &[(u64, u64, u64, State)]) -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision,
            local: peer(1),
        })
        .unwrap();
    for &(connection, identity, epoch, state) in links {
        assembler
            .push(Notice::Link {
                revision,
                connection,
                ticket: if state == State::Discovering {
                    None
                } else {
                    Some(Ticket {
                        epoch,
                        peer: peer(identity),
                    })
                },
                state,
                trusted: false,
            })
            .unwrap();
    }
    model
        .publish(assembler.push(Notice::End { revision }).unwrap().unwrap())
        .unwrap()
}
#[derive(Clone)]
struct Shared {
    status: Arc<Mutex<Status>>,
    retired: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
}
impl Shared {
    fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(Status {
                phase: Phase::Connecting,
                view: None,
                message: None,
                outcome: None,
            })),
            retired: Arc::new(AtomicBool::new(false)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
    fn set(&self, phase: Phase, view: Option<View>) {
        *self.status.lock().unwrap() = Status {
            phase,
            view,
            message: Some("test status".into()),
            outcome: Some(Outcome { id: 1, error: None }),
        };
    }
}
struct Handle(Shared);
impl Client for Handle {
    fn status(&self) -> Status {
        self.0.status.lock().unwrap().clone()
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.0.retired.load(Ordering::SeqCst)
    }
}
struct Source {
    shared: Shared,
    starts: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
}
impl Factory for Source {
    fn start(&mut self) -> Result<Box<dyn Client>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        ensure!(!self.fail.load(Ordering::SeqCst), "T924 startup failure");
        Ok(Box::new(Handle(self.shared.clone())))
    }
}
fn discovery(now: Instant) -> (Discovery, Shared, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let shared = Shared::new();
    let starts = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(false));
    let source = Source {
        shared: shared.clone(),
        starts: starts.clone(),
        fail: fail.clone(),
    };
    (
        Discovery::new(Box::new(source), Duration::from_secs(1), now).unwrap(),
        shared,
        starts,
        fail,
    )
}
#[test]
fn t924_authenticated_candidates_exclude_ambiguous_peers_and_pairing_only_links() {
    let mut model = Model::default();
    let states = [
        State::Discovering,
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ];
    let links: Vec<_> = states
        .iter()
        .enumerate()
        .map(|(i, &state)| (i as u64 + 1, i as u64 + 2, 1, state))
        .collect();
    let view = snapshot(&mut model, 1, &links);
    let selected = candidates(&view);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].connection(), 4);
    assert_eq!(selected[0].peer(), &peer(5));
    let view = snapshot(
        &mut model,
        2,
        &[
            (1, 2, 1, State::Authenticated),
            (2, 2, 2, State::Authenticated),
            (3, 3, 1, State::Authenticated),
            (4, 4, 1, State::Authenticated),
        ],
    );
    let selected = candidates(&view);
    assert_eq!(
        selected
            .iter()
            .map(Candidate::connection)
            .collect::<Vec<_>>(),
        [4, 3]
    );
    let view = snapshot(
        &mut model,
        3,
        &[(1, 2, 1, State::Authenticated), (2, 2, 2, State::Closing)],
    );
    assert_eq!(candidates(&view)[0].peer(), &peer(2));
}
#[test]
fn t924_broker_lifetime_ticket_and_connection_bound_candidate_reuse() {
    let mut model = Model::default();
    let view = snapshot(&mut model, 1, &[(1, 2, 1, State::Authenticated)]);
    let original = candidates(&view).remove(0);
    let refresh = snapshot(&mut model, 2, &[(1, 2, 1, State::Authenticated)]);
    assert!(view.same_lifetime(&refresh));
    assert!(original.same_connection(&candidates(&refresh)[0]));
    for (index, links) in [
        [(1, 2, 2, State::Authenticated)],
        [(2, 2, 1, State::Authenticated)],
        [(1, 3, 1, State::Authenticated)],
    ]
    .into_iter()
    .enumerate()
    {
        let updated = snapshot(&mut model, 3 + index as u64, &links);
        assert!(view.same_lifetime(&updated));
        assert!(!original.same_connection(&candidates(&updated)[0]));
    }
    model.reset();
    let replacement = snapshot(&mut model, 1, &[(1, 2, 1, State::Authenticated)]);
    assert!(!view.same_lifetime(&replacement));
    assert!(!original.same_connection(&candidates(&replacement)[0]));
    // Independent models have distinct lifetimes even before any reset.
    assert!(!view.same_lifetime(&snapshot(
        &mut Model::default(),
        1,
        &[(1, 2, 1, State::Authenticated)]
    )));
}
#[test]
fn t924_snapshot_link_capacity_stays_bounded_and_ordered() {
    for count in 0..=64u64 {
        let mut model = Model::default();
        let links: Vec<_> = (1..=count)
            .map(|id| (id, id + 1, 1, State::Authenticated))
            .collect();
        let view = snapshot(&mut model, 1, &links);
        let selected = candidates(&view);
        assert_eq!(selected.len(), count as usize);
        assert_eq!(
            selected
                .iter()
                .map(Candidate::connection)
                .collect::<Vec<_>>(),
            (1..=count).rev().collect::<Vec<_>>()
        );
    }
}
#[test]
fn t924_status_phases_never_turn_pairing_snapshots_into_media_authority() {
    let now = Instant::now();
    let (mut discovery, shared, starts, _) = discovery(now);
    assert!(discovery.status().is_none());
    assert!(discovery
        .poll(now - Duration::from_millis(1))
        .unwrap()
        .is_empty());
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    assert!(!discovery.retired());
    assert!(discovery.poll(now).unwrap().is_empty());
    let view = snapshot(&mut Model::default(), 1, &[(1, 2, 1, State::Authenticated)]);
    for phase in [Phase::Connecting, Phase::Ready, Phase::Busy] {
        shared.set(phase, Some(view.clone()));
        assert_eq!(
            discovery.poll(now).unwrap().len(),
            usize::from(phase != Phase::Connecting)
        );
    }
    shared.set(Phase::Ready, None);
    assert!(discovery.poll(now).unwrap().is_empty());
    assert_eq!(
        discovery.status().unwrap().message.as_deref(),
        Some("test status")
    );
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    discovery.close();
    assert!(shared.closed.load(Ordering::SeqCst));
    assert!(discovery
        .poll(now + Duration::from_secs(60))
        .unwrap()
        .is_empty());
    assert!(!discovery.retired());
    shared.retired.store(true, Ordering::SeqCst);
    assert!(discovery.retired());
}
#[test]
fn t924_failed_native_client_must_retire_before_non_sliding_retry() {
    for phase in [Phase::Closing, Phase::Closed, Phase::Failed, Phase::Ready] {
        let now = Instant::now();
        let (mut discovery, shared, starts, _) = discovery(now);
        discovery.poll(now).unwrap();
        shared.set(phase, None);
        if phase == Phase::Ready {
            shared.retired.store(true, Ordering::SeqCst);
        }
        for offset in 0..4 {
            assert!(discovery
                .poll(now + Duration::from_secs(offset))
                .unwrap()
                .is_empty());
            if phase != Phase::Ready {
                assert!(shared.closed.load(Ordering::SeqCst));
            }
        }
        if phase != Phase::Ready {
            assert_eq!(starts.load(Ordering::SeqCst), 1);
            shared.retired.store(true, Ordering::SeqCst);
            discovery.poll(now + Duration::from_secs(4)).unwrap();
            shared.retired.store(false, Ordering::SeqCst);
            shared.set(Phase::Connecting, None);
            discovery.poll(now + Duration::from_millis(4999)).unwrap();
            assert_eq!(starts.load(Ordering::SeqCst), 1);
            discovery.poll(now + Duration::from_secs(5)).unwrap();
            assert_eq!(starts.load(Ordering::SeqCst), 2);
        }
    }
}
#[test]
fn t924_start_failure_retry_bounds_and_drop_cancel_without_waiting() {
    let now = Instant::now();
    let (mut discovery, shared, starts, fail) = discovery(now);
    fail.store(true, Ordering::SeqCst);
    assert!(discovery.poll(now).is_err());
    assert!(discovery
        .poll(now + Duration::from_millis(999))
        .unwrap()
        .is_empty());
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    fail.store(false, Ordering::SeqCst);
    discovery.poll(now + Duration::from_secs(1)).unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 2);
    drop(discovery);
    assert!(shared.closed.load(Ordering::SeqCst));
    let (mut unused, _, _, _) = self::discovery(now);
    unused.close();
    unused.close();
    assert!(unused.retired());
    for retry in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        let source = Source {
            shared: Shared::new(),
            starts: Arc::new(AtomicUsize::new(0)),
            fail: Arc::new(AtomicBool::new(false)),
        };
        assert!(Discovery::new(Box::new(source), retry, now).is_err());
    }
}
#[test]
fn t924_counter_and_phase_boundaries_have_no_aliases() {
    let max = blent_config::connection::wire::MAX_COUNTER;
    for connection in [1, 2, max] {
        for epoch in [1, 2, max] {
            let mut model = Model::default();
            let view = snapshot(
                &mut model,
                1,
                &[(connection, 2, epoch, State::Authenticated)],
            );
            let candidate = &candidates(&view)[0];
            assert_eq!(candidate.connection(), connection);
            let action = view
                .prepare(Command::Disconnect {
                    connection,
                    ticket: Some(Ticket {
                        peer: peer(2),
                        epoch,
                    }),
                })
                .unwrap();
            assert!(model.command(&action).is_ok());
        }
    }
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
fn t924_exhausted_clock_cannot_spin_startup_or_reuse_native_owner() {
    let now = latest_instant(Instant::now());
    assert!(now.checked_add(Duration::from_secs(1)).is_none());
    let (mut discovery, _, starts, fail) = self::discovery(now);
    fail.store(true, Ordering::SeqCst);
    assert!(discovery.poll(now).is_err());
    for _ in 0..64 {
        assert!(discovery.poll(now).unwrap().is_empty());
    }
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    let (mut discovery, shared, starts, _) = self::discovery(now);
    discovery.poll(now).unwrap();
    shared.retired.store(true, Ordering::SeqCst);
    discovery.poll(now).unwrap();
    for _ in 0..64 {
        assert!(discovery.poll(now).unwrap().is_empty());
    }
    assert_eq!(starts.load(Ordering::SeqCst), 1);
}

#[test]
fn t924_retired_client_failure_remains_visible_during_retry() {
    let now = Instant::now();
    for startup_failure in [false, true] {
        let (mut discovery, shared, _, fail) = self::discovery(now);
        if startup_failure {
            fail.store(true, Ordering::SeqCst);
            assert!(discovery.poll(now).is_err());
        } else {
            discovery.poll(now).unwrap();
            shared.set(
                Phase::Failed,
                Some(snapshot(
                    &mut Model::default(),
                    1,
                    &[(1, 2, 1, State::Authenticated)],
                )),
            );
            shared.status.lock().unwrap().message = Some("é".repeat(2048));
            shared.retired.store(true, Ordering::SeqCst);
            discovery.poll(now).unwrap();
        }
        let status = discovery
            .status()
            .expect("T924 failure vanished during retry");
        assert_eq!(status.phase, Phase::Failed);
        assert!(
            status.view.is_none(),
            "retired source retained obsolete candidates"
        );
        assert!(status
            .message
            .as_ref()
            .is_some_and(|message| !message.is_empty() && message.chars().count() <= 512));
    }
}

#[test]
fn t924_completed_native_client_cannot_keep_ready_status() {
    let now = Instant::now();
    let (mut discovery, shared, _, _) = discovery(now);
    discovery.poll(now).unwrap();
    shared.set(
        Phase::Ready,
        Some(snapshot(
            &mut Model::default(),
            1,
            &[(1, 2, 1, State::Authenticated)],
        )),
    );
    shared.retired.store(true, Ordering::SeqCst);
    assert!(discovery.poll(now).unwrap().is_empty());
    assert_eq!(
        discovery.status().unwrap().phase,
        Phase::Closed,
        "T924 completed native source still appears ready"
    );
}

#[test]
fn t924_ambiguity_blocks_new_admission_without_revoking_exact_existing_link() {
    let now = Instant::now();
    let (mut discovery, shared, _, _) = discovery(now);
    let mut model = Model::default();
    shared.set(
        Phase::Ready,
        Some(snapshot(&mut model, 1, &[(1, 2, 1, State::Authenticated)])),
    );
    let candidate = discovery.poll(now).unwrap().remove(0);
    assert!(discovery.current(&candidate));
    shared.set(
        Phase::Ready,
        Some(snapshot(
            &mut model,
            2,
            &[
                (1, 2, 1, State::Authenticated),
                (2, 2, 2, State::Authenticated),
            ],
        )),
    );
    assert!(discovery.poll(now).unwrap().is_empty());
    assert!(discovery.current(&candidate));
    shared.set(
        Phase::Busy,
        Some(snapshot(&mut model, 3, &[(1, 2, 1, State::Authenticated)])),
    );
    assert!(discovery.current(&candidate));
    for (index, links) in [
        vec![],
        vec![(1, 2, 1, State::Closing)],
        vec![(1, 2, 2, State::Authenticated)],
        vec![(1, 3, 1, State::Authenticated)],
    ]
    .into_iter()
    .enumerate()
    {
        shared.set(
            Phase::Ready,
            Some(snapshot(&mut model, 4 + index as u64, &links)),
        );
        assert!(!discovery.current(&candidate));
    }
    model.reset();
    shared.set(
        Phase::Ready,
        Some(snapshot(&mut model, 1, &[(1, 2, 1, State::Authenticated)])),
    );
    assert!(!discovery.current(&candidate));
    shared.set(Phase::Connecting, None);
    assert!(!discovery.current(&candidate));
    shared.set(Phase::Ready, None);
    assert!(!discovery.current(&candidate));
    shared.retired.store(true, Ordering::SeqCst);
    assert!(!discovery.current(&candidate));
    discovery.close();
    assert!(!discovery.current(&candidate));
    let (unused, _, _, _) = self::discovery(now);
    assert!(!unused.current(&candidate));
}
