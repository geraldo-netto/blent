use super::*;
use crate::connection::wire::MAX_COUNTER;

fn request() -> Request {
    Request {
        peer: PeerId::parse("ab".repeat(32)).unwrap(),
        resource: Resource::DisplaySession,
        credential: Credential::new([7; 32]),
        profile: Profile::Display,
    }
}
fn route() -> RouteId {
    RouteId {
        owner: 1,
        revision: 2,
    }
}
fn model() -> Model {
    Model::new(request()).unwrap()
}
fn registered() -> Model {
    let mut model = model();
    assert!(model.outgoing() == Some(request().message()));
    model
        .receive(Message::Registered {
            request: 1,
            route: route(),
        })
        .unwrap();
    model
}
fn progress(model: &mut Model, request: u64, state: State) {
    model
        .receive(Message::Status {
            request,
            route: route(),
            state,
        })
        .unwrap();
}

#[test]
fn t910_admission_requires_current_route_status_and_retirement_requires_exact_proof() {
    let mut model = model();
    assert_eq!(model.status().phase, Phase::Starting);
    assert!(!model.status().remote_retired);
    assert!(model.outgoing() == Some(request().message()));
    assert_eq!(model.status().phase, Phase::Registering);
    assert!(model.outgoing().is_none());
    model
        .receive(Message::Registered {
            request: 1,
            route: route(),
        })
        .unwrap();
    assert_eq!(model.status().phase, Phase::Queued);
    progress(&mut model, 1, State::Queued);
    progress(&mut model, 1, State::Active);
    assert_eq!(model.status().phase, Phase::Active);
    assert!(model.outgoing().is_none());
    model.close();
    assert_eq!(model.status().phase, Phase::Closing);
    assert!(!model.status().remote_retired);
    assert!(
        model.outgoing()
            == Some(Message::Unregister {
                request: 2,
                route: route()
            })
    );
    assert!(model.outgoing().is_none());
    progress(&mut model, 2, State::Closing);
    progress(&mut model, 2, State::Retired);
    assert_eq!(model.status().phase, Phase::Retired);
    assert!(model.status().remote_retired);
    model.close();
    assert!(model.outgoing().is_none());
}
#[test]
fn t910_cancel_before_exposure_opens_nothing_and_cancel_during_start_waits_route() {
    let mut untouched = model();
    untouched.close();
    untouched.close();
    assert!(untouched.outgoing().is_none());
    assert!(untouched.status().remote_retired);
    let mut opening = model();
    opening.outgoing();
    opening.close();
    assert!(!opening.status().remote_retired);
    assert!(opening.outgoing().is_none());
    opening
        .receive(Message::Registered {
            request: 1,
            route: route(),
        })
        .unwrap();
    assert_eq!(opening.status().phase, Phase::Closing);
    assert!(
        opening.outgoing()
            == Some(Message::Unregister {
                request: 2,
                route: route()
            })
    );
    progress(&mut opening, 1, State::Queued);
    progress(&mut opening, 1, State::Active); // Already queued before unregister.
    assert_eq!(opening.status().phase, Phase::Closing);
    progress(&mut opening, 1, State::Retired); // Proof may precede handling unregister.
    assert!(opening.status().remote_retired);
}
#[test]
fn t910_failed_status_and_native_loss_never_release_remote_ownership() {
    for failure in [Failure::Native, Failure::TimedOut, Failure::Protocol] {
        let mut model = registered();
        progress(&mut model, 1, State::Active);
        model.fail(failure);
        assert!(!model.status().remote_retired);
        assert_eq!(model.status().phase, Phase::Failed);
        assert!(model.outgoing().is_some());
        progress(&mut model, 2, State::Failed);
        progress(&mut model, 2, State::Closing);
        assert_eq!(model.status().failure, Some(failure));
        assert!(!model.status().remote_retired);
        progress(&mut model, 2, State::Retired);
        assert!(model.status().remote_retired);
        assert_eq!(model.status().failure, Some(failure));
    }
    let mut native = model();
    native.fail(Failure::Native);
    assert!(native.status().remote_retired);
    assert!(native.outgoing().is_none());
}
#[test]
fn t910_rejection_is_retirement_only_before_registration_was_owned() {
    for reason in [
        Rejection::Invalid,
        Rejection::Unavailable,
        Rejection::Busy,
        Rejection::Stale,
        Rejection::Denied,
        Rejection::Failed,
    ] {
        let mut model = model();
        model.outgoing();
        model
            .receive(Message::Rejected { request: 1, reason })
            .unwrap();
        assert!(model.status().remote_retired);
        assert_eq!(model.status().failure, Some(Failure::Rejected(reason)));
        let mut owned = registered();
        owned.close();
        owned.outgoing();
        owned
            .receive(Message::Rejected { request: 2, reason })
            .unwrap();
        assert!(!owned.status().remote_retired);
        assert_eq!(owned.status().phase, Phase::Failed);
        progress(&mut owned, 1, State::Retired);
        assert!(owned.status().remote_retired);
    }
}
#[test]
fn t910_peer_initiated_close_and_failed_admission_revoke_before_native_cleanup() {
    let mut model = registered();
    progress(&mut model, 1, State::Active);
    progress(&mut model, 1, State::Closing);
    assert_eq!(model.status().phase, Phase::Closing);
    assert!(model.outgoing().is_some());
    progress(&mut model, 1, State::Closing);
    progress(&mut model, 1, State::Failed);
    assert_eq!(model.status().failure, Some(Failure::Resource));
    assert!(!model.status().remote_retired);
    progress(&mut model, 2, State::Retired);
    assert!(model.status().remote_retired);
}
#[test]
fn t910_malformed_foreign_and_replayed_responses_cannot_grant_or_retire_resources() {
    let bad = [
        Message::Registered {
            request: 2,
            route: route(),
        },
        Message::Status {
            request: 1,
            route: RouteId {
                owner: 9,
                revision: 2,
            },
            state: State::Retired,
        },
        Message::Status {
            request: 2,
            route: route(),
            state: State::Active,
        },
        Message::Rejected {
            request: 1,
            reason: Rejection::Failed,
        },
        Message::Rejected {
            request: 2,
            reason: Rejection::Failed,
        },
        request().message(),
    ];
    for message in bad {
        let mut model = registered();
        progress(&mut model, 1, State::Active);
        assert!(model.receive(message).is_err());
        assert_eq!(model.status().phase, Phase::Failed);
        assert!(!model.status().remote_retired);
        assert_eq!(model.status().route, Some(route()));
        assert!(model.outgoing().is_some());
    }
    let mut model = registered();
    progress(&mut model, 1, State::Retired);
    assert!(model
        .receive(Message::Registered {
            request: 1,
            route: route()
        })
        .is_err());
    assert!(model.status().remote_retired);
    assert_ne!(model.status().phase, Phase::Active);
}
#[test]
fn t910_invalid_and_extreme_counters_reject_before_retirement_proof() {
    for value in [0, 3, MAX_COUNTER, MAX_COUNTER + 1, u64::MAX] {
        let mut model = registered();
        let status = Message::Status {
            request: value,
            route: route(),
            state: State::Retired,
        };
        assert!(model.receive(status).is_err());
        assert!(!model.status().remote_retired);
    }
    for value in [0, MAX_COUNTER + 1, u64::MAX] {
        let mut model = model();
        model.outgoing();
        assert!(model
            .receive(Message::Registered {
                request: 1,
                route: RouteId {
                    owner: value,
                    revision: 1
                }
            })
            .is_err());
        assert!(!model.status().remote_retired);
        assert!(model.status().route.is_none());
    }
    let mut model = model();
    model.outgoing();
    let edge = RouteId {
        owner: MAX_COUNTER,
        revision: MAX_COUNTER,
    };
    model
        .receive(Message::Registered {
            request: 1,
            route: edge,
        })
        .unwrap();
    model.close();
    assert!(
        model.outgoing()
            == Some(Message::Unregister {
                request: 2,
                route: edge
            })
    );
}
#[test]
fn t910_all_bounded_status_pairs_preserve_terminal_and_failed_admission() {
    let states = [
        State::Queued,
        State::Active,
        State::Closing,
        State::Failed,
        State::Retired,
    ];
    for first in states {
        for second in states {
            let mut model = registered();
            progress(&mut model, 1, first);
            let accepted = model
                .receive(Message::Status {
                    request: 1,
                    route: route(),
                    state: second,
                })
                .is_ok();
            let forbidden = first == State::Retired
                || (first == State::Active && second == State::Queued)
                || (matches!(first, State::Closing | State::Failed)
                    && matches!(second, State::Queued | State::Active));
            assert_ne!(accepted, forbidden);
            if matches!(first, State::Closing | State::Failed | State::Retired) {
                assert_ne!(model.status().phase, Phase::Active);
            }
        }
    }
}
#[test]
fn t910_invalid_profile_and_response_before_exposure_do_not_start_native_work() {
    let mut bad = request();
    bad.resource = Resource::Camera;
    assert!(Model::new(bad).is_err());
    let mut model = model();
    assert!(model
        .receive(Message::Registered {
            request: 1,
            route: route()
        })
        .is_err());
    assert!(model.status().remote_retired);
    assert!(model.outgoing().is_none());
}

struct Broker(bool);
impl BrokerLifetime for Broker {
    fn exited(&self) -> bool {
        self.0
    }
}
#[test]
fn t1007_original_exit_never_restores_authority_or_uses_socket_failure_as_proof() {
    for failure in [
        Failure::Native,
        Failure::TimedOut,
        Failure::Protocol,
        Failure::Resource,
    ] {
        let mut model = registered();
        progress(&mut model, 1, State::Active);
        model.fail(failure);
        model.observe_broker_exit(&Broker(false));
        assert!(!model.status().remote_retired);
        assert_eq!(model.status().failure, Some(failure));
        model.observe_broker_exit(&Broker(true));
        assert!(model.status().remote_retired);
        assert_eq!(model.status().failure, Some(failure));
        assert!(!model.status().live());
        assert!(model.outgoing().is_none());
        model.observe_broker_exit(&Broker(true));
        assert!(!model.status().live());
    }
    let mut partial = model();
    partial.outgoing(); // Death before Registered still proves no remote owner.
    partial.observe_broker_exit(&Broker(true));
    assert!(partial.status().remote_retired);
    assert_eq!(partial.status().failure, Some(Failure::Native));
    let mut untouched = model();
    untouched.close();
    untouched.observe_broker_exit(&Broker(true));
    assert!(untouched.status().failure.is_none());
}
