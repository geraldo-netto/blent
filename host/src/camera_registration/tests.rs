use super::test_support::*;
use super::*;
use crate::display_test_support::peer;
use crate::media_registration::State;
use blent_config::connection::{
    control::State as LinkState,
    control_client::Phase as ControlPhase,
    registration_client::{Failure, Status},
    wire::MAX_COUNTER,
};
use blent_config::connection::{registration_client::Phase, routes::RouteId};
use std::sync::atomic::Ordering;

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
