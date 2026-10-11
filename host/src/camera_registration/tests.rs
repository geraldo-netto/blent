use super::*;
use crate::display_test_support::{peer, Backend, Calls, Route};
use blent_config::connection::{
    control::{Notice, State as LinkState},
    control_client::{
        Client as ControlClient, Factory as ControlFactory, Phase as ControlPhase,
        Status as ControlStatus, SubmitError,
    },
    control_view::{Action, Assembler, Limits, Model, View},
    pairing::Ticket,
    registration_client::{Failure, Status},
    wire::MAX_COUNTER,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

fn view(links: &[(u64, u64, LinkState)]) -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 1,
            local: peer(1),
        })
        .unwrap();
    for &(connection, identity, state) in links {
        assembler
            .push(Notice::Link {
                revision: 1,
                connection,
                ticket: Some(Ticket {
                    epoch: 1,
                    peer: peer(identity),
                }),
                state,
                trusted: false,
            })
            .unwrap();
    }
    Model::default()
        .publish(
            assembler
                .push(Notice::End { revision: 1 })
                .unwrap()
                .unwrap(),
        )
        .unwrap()
}

struct Control {
    status: Mutex<ControlStatus>,
    closed: AtomicBool,
    done: AtomicBool,
    starts: AtomicUsize,
    fail: AtomicBool,
}
#[derive(Clone)]
struct ControlHandle(Arc<Control>);
impl std::ops::Deref for ControlHandle {
    type Target = Control;
    fn deref(&self) -> &Control {
        &self.0
    }
}
impl ControlClient for ControlHandle {
    fn status(&self) -> ControlStatus {
        self.status.lock().unwrap().clone()
    }
    fn submit(&self, _: Action) -> std::result::Result<u64, SubmitError> {
        Err(SubmitError::Closed)
    }
    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
    fn retired(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }
}
impl ControlFactory for ControlHandle {
    fn start(&mut self) -> Result<Box<dyn ControlClient>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        ensure!(
            !self.fail.load(Ordering::SeqCst),
            "unpublished control startup failure"
        );
        Ok(Box::new(self.clone()))
    }
}
struct Rig {
    source: Arc<Control>,
    calls: Arc<Mutex<Calls>>,
    now: Instant,
}
impl Rig {
    fn new(links: &[(u64, u64, LinkState)]) -> Self {
        Self {
            source: Arc::new(Control {
                status: Mutex::new(ControlStatus {
                    phase: ControlPhase::Ready,
                    view: Some(view(links)),
                    message: None,
                    outcome: None,
                }),
                closed: AtomicBool::new(false),
                done: AtomicBool::new(false),
                starts: AtomicUsize::new(0),
                fail: AtomicBool::new(false),
            }),
            calls: Arc::new(Mutex::new(Calls::default())),
            now: Instant::now(),
        }
    }
    fn owner(&self, profile: &CameraProfile, token: &str) -> Result<CameraRegistration> {
        CameraRegistration::new(
            profile,
            token,
            Discovery::new(
                Box::new(ControlHandle(self.source.clone())),
                Duration::from_secs(1),
                self.now,
            )?,
            Box::new(Backend(self.calls.clone())),
        )
    }
    fn client(&self) -> Arc<Route> {
        self.calls.lock().unwrap().clients[0].clone()
    }
    fn complete(&self) {
        self.client().set(Phase::Retired);
        self.client().done.store(true, Ordering::SeqCst);
        self.source.done.store(true, Ordering::SeqCst);
    }
}
fn profile() -> CameraProfile {
    CameraProfile {
        usb_peer: Some(peer(2).as_str().into()),
        ..Default::default()
    }
}
fn authenticated() -> Rig {
    Rig::new(&[(1, 2, LinkState::Authenticated)])
}

#[test]
fn t969_explicit_peer_ambiguity_and_independent_admission() {
    let rig = Rig::new(&[
        (1, 3, LinkState::Authenticated),
        (2, 2, LinkState::Authenticated),
        (3, 2, LinkState::Authenticated),
    ]);
    let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    owner.poll(rig.now).unwrap();
    assert_eq!(owner.state(), State::Waiting);
    assert!(rig.calls.lock().unwrap().requests.is_empty());
    rig.source.status.lock().unwrap().view = Some(view(&[
        (1, 3, LinkState::Authenticated),
        (2, 2, LinkState::Authenticated),
    ]));
    owner.poll(rig.now).unwrap();
    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.requests.len(), 1);
    assert_eq!(calls.requests[0].peer, peer(2));
    assert_eq!(calls.requests[0].resource, Resource::Camera);
    assert_eq!(calls.requests[0].credential.as_bytes(), &[0xab; 32]);
    assert_eq!(
        calls.requests[0].profile,
        Profile::Camera(profile().invitation_profile().unwrap())
    );
    drop(calls);
    assert_eq!(owner.state(), State::Registering);
    rig.client().set(Phase::Queued);
    owner.poll(rig.now).unwrap();
    assert_eq!(owner.state(), State::Registering);
    rig.client().set(Phase::Active);
    owner.poll(rig.now).unwrap();
    assert_eq!(owner.state(), State::Active);
    owner.close();
    assert_eq!(owner.state(), State::Retiring);
    rig.client().set(Phase::Retired);
    rig.client().done.store(true, Ordering::SeqCst);
    assert!(!owner.retired());
    rig.source.done.store(true, Ordering::SeqCst);
    owner.poll(rig.now).unwrap();
    assert_eq!(owner.state(), State::Retired);
}

#[test]
fn t969_equal_tickets_in_replacement_source_cannot_revive_registration() {
    let rig = authenticated();
    let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    owner.poll(rig.now).unwrap();
    rig.client().set(Phase::Active);
    owner.poll(rig.now).unwrap();
    rig.source.status.lock().unwrap().view = Some(view(&[(1, 2, LinkState::Authenticated)]));
    assert!(owner.poll(rig.now).is_err());
    assert!(rig.client().closed.load(Ordering::SeqCst));
    assert!(rig.source.closed.load(Ordering::SeqCst));
    for _ in 0..64 {
        owner.poll(rig.now).unwrap();
    }
    assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
    assert!(!owner.retired());
    rig.complete();
    assert!(owner.retired());
}

fn malformed() -> Vec<Status> {
    let valid = Status {
        phase: Phase::Active,
        route: Some(RouteId {
            owner: 1,
            revision: 1,
        }),
        failure: None,
        remote_retired: false,
    };
    let mut cases = vec![
        Status {
            route: None,
            ..valid.clone()
        },
        Status {
            phase: Phase::Starting,
            ..valid.clone()
        },
        Status {
            failure: Some(Failure::Native),
            ..valid.clone()
        },
        Status {
            remote_retired: true,
            ..valid.clone()
        },
    ];
    for value in [0, MAX_COUNTER + 1, u64::MAX] {
        cases.push(Status {
            route: Some(RouteId {
                owner: value,
                revision: 1,
            }),
            ..valid.clone()
        });
        cases.push(Status {
            route: Some(RouteId {
                owner: 1,
                revision: value,
            }),
            ..valid.clone()
        });
    }
    for phase in [Phase::Closing, Phase::Failed, Phase::Retired] {
        cases.push(Status {
            phase,
            ..valid.clone()
        });
    }
    cases
}

#[test]
fn t969_invalid_status_never_releases_actual_cleanup_or_admits_capture() {
    for status in malformed() {
        let rig = authenticated();
        let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
        owner.poll(rig.now).unwrap();
        *rig.client().status.lock().unwrap() = status;
        assert!(owner.poll(rig.now).is_err());
        assert_eq!(owner.state(), State::Retiring);
        assert!(!owner.retired());
        rig.complete();
        assert_eq!(owner.state(), State::Retired);
    }
}

#[test]
fn t969_changed_routes_and_admission_rollback_close_original_only() {
    for change_route in [false, true] {
        let rig = authenticated();
        let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
        owner.poll(rig.now).unwrap();
        rig.client().set(Phase::Active);
        owner.poll(rig.now).unwrap();
        if change_route {
            rig.client().status.lock().unwrap().route = Some(RouteId {
                owner: 2,
                revision: 1,
            });
        } else {
            rig.client().set(Phase::Queued);
        }
        assert!(owner.poll(rig.now).is_err());
        assert!(rig.client().closed.load(Ordering::SeqCst));
        assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
        rig.complete();
        assert!(owner.retired());
    }
}

#[test]
fn t969_closed_owner_and_unpublished_startup_failures_never_start_replacements() {
    let rig = authenticated();
    let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    owner.close();
    owner.poll(rig.now).unwrap();
    assert!(owner.retired());
    assert!(rig.calls.lock().unwrap().requests.is_empty());
    rig.source.fail.store(true, Ordering::SeqCst);
    let mut failed = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    assert!(failed.poll(rig.now).is_err());
    assert!(failed.retired());
    rig.source.fail.store(false, Ordering::SeqCst);
    rig.calls.lock().unwrap().fail = true;
    let mut unpublished = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    assert!(unpublished.poll(rig.now).is_err());
    assert!(!unpublished.retired());
    rig.source.done.store(true, Ordering::SeqCst);
    assert!(unpublished.retired());
}

#[test]
fn t969_invalid_profile_target_and_token_fail_before_native_start() {
    let rig = authenticated();
    let bad_profiles = [
        CameraProfile::default(),
        CameraProfile {
            serial: Some("adb".into()),
            ..profile()
        },
        CameraProfile {
            width: 0,
            ..profile()
        },
        CameraProfile {
            fps: u32::MAX,
            ..profile()
        },
        CameraProfile {
            usb_peer: Some("invalid".into()),
            ..Default::default()
        },
    ];
    for profile in bad_profiles {
        assert!(rig.owner(&profile, &"ab".repeat(32)).is_err());
    }
    for length in 0..128 {
        if length != 64 {
            assert!(rig.owner(&profile(), &"a".repeat(length)).is_err());
        }
    }
    for invalid in ['A', 'g', ';', '\0', 'é'] {
        for offset in 0..64 {
            let mut bytes = "a".repeat(64);
            bytes.replace_range(offset..offset + 1, &invalid.to_string());
            assert!(rig.owner(&profile(), &bytes).is_err());
        }
    }
    assert_eq!(rig.source.starts.load(Ordering::SeqCst), 0);
    assert!(rig.calls.lock().unwrap().requests.is_empty());
}

#[test]
fn t969_retired_or_lost_source_and_drop_close_without_forgetting_native_clients() {
    for retire_client in [false, true] {
        let rig = authenticated();
        let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
        owner.poll(rig.now).unwrap();
        if retire_client {
            rig.client().done.store(true, Ordering::SeqCst);
        } else {
            rig.source.status.lock().unwrap().phase = ControlPhase::Failed;
        }
        assert!(owner.poll(rig.now).is_err());
        assert_eq!(owner.state(), State::Retiring);
        rig.complete();
        assert!(owner.retired());
    }
    let rig = authenticated();
    let mut owner = rig.owner(&profile(), &"ab".repeat(32)).unwrap();
    owner.poll(rig.now).unwrap();
    drop(owner);
    assert!(rig.client().closed.load(Ordering::SeqCst));
    assert!(rig.source.closed.load(Ordering::SeqCst));
    assert!(!rig.client().done.load(Ordering::SeqCst));
}

#[test]
fn t969_distinct_invocations_preserve_fresh_supplied_credentials() {
    let mut credentials = Vec::new();
    for token in ["01".repeat(32), "02".repeat(32)] {
        let rig = authenticated();
        let mut owner = rig.owner(&profile(), &token).unwrap();
        owner.poll(rig.now).unwrap();
        owner.close();
        rig.complete();
        assert!(owner.retired());
        credentials.push(*rig.calls.lock().unwrap().requests[0].credential.as_bytes());
    }
    assert_ne!(credentials[0], credentials[1]);
}
