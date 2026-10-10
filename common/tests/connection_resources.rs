//! T851: authority, ordering and actual-retirement contract.
use blent_config::connection::{
    management::{Key, Outcome, Resource},
    pairing::{PeerId, Ticket as Connection},
    resources::*,
};
fn connection(epoch: u64) -> Connection {
    Connection {
        epoch,
        peer: PeerId::parse("ab".repeat(32)).unwrap(),
    }
}
fn pair() -> (Resources, Resources) {
    (
        Resources::new(Role::Host, connection(1)).unwrap(),
        Resources::new(Role::Tablet, connection(1)).unwrap(),
    )
}
fn offer(host: &mut Resources, tablet: &mut Resources, resource: Resource) -> (Lease, Lease) {
    let host_lease = host.begin(resource, 11, 0, 100).unwrap();
    let tablet_lease = tablet.offer(host_lease.key, 22, 0, 100).unwrap();
    (host_lease, tablet_lease)
}
fn active(host: &mut Resources, tablet: &mut Resources, resource: Resource) -> (Lease, Lease) {
    let (h, t) = offer(host, tablet, resource);
    tablet.prepared(&t, 1).unwrap();
    host.prepared(&h, 2).unwrap();
    host.start_commit(&h, 3).unwrap();
    assert_eq!(host.commit(&h, 4).unwrap(), Install::Activate);
    assert_eq!(tablet.commit(&t, 5).unwrap(), Install::Activate);
    assert!(tablet.admitted(&t, 6).unwrap());
    assert!(host.admitted(&h, 7).unwrap());
    assert!(host.current(&h));
    assert!(tablet.current(&t));
    (h, t)
}
fn finish(host: &mut Resources, tablet: &mut Resources, h: &Lease, t: &Lease, installed: bool) {
    host.revoke(h).unwrap();
    host.revoke_written(h).unwrap();
    tablet.peer_revoke(t).unwrap();
    assert!(tablet.acknowledge(t).is_err());
    tablet.native_retired(t).unwrap();
    if installed {
        tablet.channels_retired(t).unwrap();
    }
    tablet.acknowledge(t).unwrap();
    host.acknowledge(h).unwrap();
    assert!(host.finish(h).is_err());
    host.native_retired(h).unwrap();
    if installed {
        host.channels_retired(h).unwrap();
    }
    tablet.finish(t).unwrap();
    host.finish(h).unwrap();
}
#[test]
fn t851_independent_resources_revoke_before_retirement_and_reuse() {
    let (mut h, mut t) = pair();
    let (display, display_t) = active(&mut h, &mut t, Resource::DisplaySession);
    let (camera, camera_t) = active(&mut h, &mut t, Resource::Camera);
    assert!(h.begin(Resource::Camera, 11, 0, 100).is_err());
    assert!(h.revoke(&camera).unwrap());
    assert!(!h.current(&camera));
    assert!(h.current(&display));
    assert!(t.current(&display_t));
    finish(&mut h, &mut t, &camera, &camera_t, true);
    let (next, next_t) = offer(&mut h, &mut t, Resource::Camera);
    assert_eq!(next.key.generation, camera.key.generation + 1);
    assert!(!h.current(&camera));
    assert!(h.native_retired(&camera).is_err());
    finish(&mut h, &mut t, &next, &next_t, false);
    finish(&mut h, &mut t, &display, &display_t, true);
}
#[test]
fn t851_cancel_crossing_commit_installs_drain_without_reviving_authority() {
    let (mut h, mut t) = pair();
    let (a, b) = offer(&mut h, &mut t, Resource::Camera);
    t.prepared(&b, 1).unwrap();
    h.prepared(&a, 2).unwrap();
    h.start_commit(&a, 3).unwrap();
    t.revoke(&b).unwrap();
    h.peer_revoke(&a).unwrap();
    assert!(h.revoke_written(&a).is_err());
    assert_eq!(h.commit(&a, 4).unwrap(), Install::Drain);
    assert_eq!(t.commit(&b, 5).unwrap(), Install::Drain);
    assert!(t.admitted(&b, 6).is_err());
    assert!(!h.current(&a));
    finish(&mut h, &mut t, &a, &b, true);
}
#[test]
fn t851_denial_and_stale_consent_never_consume_wire_generation() {
    let (mut h, mut t) = pair();
    for outcome in [
        Outcome::Unavailable,
        Outcome::PermissionDenied,
        Outcome::Busy,
        Outcome::Failed,
    ] {
        let (a, b) = offer(&mut h, &mut t, Resource::Microphone);
        assert!(t.permission_required(&b, 1).unwrap());
        assert!(h.permission_required(&a, 2).unwrap());
        assert!(t.permission_required(&b, 3).is_err());
        t.reject(&b, outcome).unwrap();
        h.reject(&a, outcome).unwrap();
        assert!(t.prepared(&b, 4).is_err());
        assert!(h.start_commit(&a, 4).is_err());
        finish(&mut h, &mut t, &a, &b, false);
        assert_eq!(a.key.generation, 1);
    }
}
#[test]
fn t851_deadline_is_absolute_and_disconnect_does_not_prove_retirement() {
    let (mut h, mut t) = pair();
    let (a, b) = offer(&mut h, &mut t, Resource::Speakers);
    t.prepared(&b, 1).unwrap();
    h.prepared(&a, 2).unwrap();
    h.start_commit(&a, 3).unwrap();
    assert!(h.expire(99).unwrap().iter().all(Option::is_none));
    assert_eq!(h.expire(100).unwrap()[3].as_ref(), Some(&a));
    assert_eq!(h.commit(&a, 101).unwrap(), Install::Drain);
    assert_eq!(t.commit(&b, 100).unwrap(), Install::Drain);
    assert!(t.prepared(&b, 101).is_err());
    h.disconnect();
    t.disconnect();
    assert!(h.finish(&a).is_err());
    assert!(t.finish(&b).is_err());
    h.native_retired(&a).unwrap();
    t.native_retired(&b).unwrap();
    assert!(h.finish(&a).is_err());
    h.channels_retired(&a).unwrap();
    t.channels_retired(&b).unwrap();
    h.finish(&a).unwrap();
    t.finish(&b).unwrap();
    assert!(h.begin(Resource::Speakers, 11, 0, 100).is_err());
    let mut next = Resources::new(Role::Host, connection(2)).unwrap();
    let fresh = next.begin(Resource::Speakers, 11, 0, 100).unwrap();
    assert_ne!(fresh, a);
    assert!(next.prepared(&a, 1).is_err());
}
#[test]
fn t851_late_prepared_permission_and_admission_cannot_restore_authority() {
    let (mut h, mut t) = pair();
    let (a, b) = offer(&mut h, &mut t, Resource::Camera);
    t.permission_required(&b, 1).unwrap();
    t.prepared(&b, 2).unwrap();
    h.revoke(&a).unwrap();
    h.revoke_written(&a).unwrap();
    assert!(!h.permission_required(&a, 3).unwrap());
    assert!(!h.prepared(&a, 4).unwrap());
    assert!(h.start_commit(&a, 5).is_err());
    t.peer_revoke(&b).unwrap();
    assert!(t.commit(&b, 6).is_err());
    t.native_retired(&b).unwrap();
    assert!(t.ready_to_ack(&b).unwrap());
    t.acknowledge(&b).unwrap();
    h.acknowledge(&a).unwrap();
    h.native_retired(&a).unwrap();
    h.finish(&a).unwrap();
    t.finish(&b).unwrap();
    let (a, b) = offer(&mut h, &mut t, Resource::Camera);
    t.prepared(&b, 1).unwrap();
    h.prepared(&a, 2).unwrap();
    h.start_commit(&a, 3).unwrap();
    h.commit(&a, 4).unwrap();
    t.commit(&b, 5).unwrap();
    t.admitted(&b, 6).unwrap();
    h.revoke(&a).unwrap();
    assert!(!h.admitted(&a, 7).unwrap());
    assert!(!h.current(&a));
    finish(&mut h, &mut t, &a, &b, true);
}
#[test]
fn t851_invalid_bounds_roles_keys_and_out_of_order_events() {
    assert!(Resources::new(Role::Host, connection(0)).is_err());
    assert!(Resources::new(Role::Tablet, connection(u64::MAX)).is_err());
    let (mut h, mut t) = pair();
    for (owner, now, timeout) in [
        (0, 0, 1),
        (u64::MAX, 0, 1),
        (1, -1, 1),
        (1, 0, 0),
        (1, 0, 600001),
        (1, i64::MAX, 1),
    ] {
        assert!(h.begin(Resource::Camera, owner, now, timeout).is_err());
    }
    let key = Key {
        resource: Resource::Camera,
        request: 1,
        generation: 1,
    };
    assert!(h.offer(key, 1, 0, 1).is_err());
    assert!(t.begin(Resource::Camera, 1, 0, 1).is_err());
    for key in [
        Key { request: 0, ..key },
        Key {
            generation: 0,
            ..key
        },
        Key {
            request: u64::MAX,
            ..key
        },
        Key {
            generation: u64::MAX,
            ..key
        },
        Key {
            generation: 2,
            ..key
        },
    ] {
        assert!(t.offer(key, 1, 0, 1).is_err());
    }
    let (a, b) = offer(&mut h, &mut t, Resource::Camera);
    assert_eq!(a.key.request, 1); // Rejections did not allocate IDs.
    assert!(!h.ready_to_ack(&a).unwrap());
    assert!(h.commit(&a, 1).is_err());
    assert!(t.commit(&b, 1).is_err());
    assert!(h.admitted(&a, 1).is_err());
    assert!(h.start_commit(&a, 1).is_err());
    assert!(h.revoke_written(&a).is_err());
    assert!(t.revoke_written(&b).is_err());
    assert!(h.acknowledge(&a).is_err());
    assert!(h.native_retired(&a).is_err());
    assert!(h.channels_retired(&a).is_err());
    assert!(h.expire(-1).is_err());
    for outcome in [
        Outcome::Admitted,
        Outcome::Revoked,
        Outcome::PermissionRequired,
    ] {
        assert!(h.reject(&a, outcome).is_err());
    }
    let stale = Lease {
        owner: 99,
        ..a.clone()
    };
    assert!(h.prepared(&stale, 1).is_err());
    assert!(!h.current(&stale));
    assert!(h.prepared(&a, -1).is_err());
    assert!(h.permission_required(&a, -1).is_err());
    t.prepared(&b, 1).unwrap();
    h.prepared(&a, 2).unwrap();
    assert!(t.start_commit(&b, 3).is_err());
    assert!(h.prepared(&a, 3).is_err());
    assert!(h.reject(&a, Outcome::Failed).is_err());
    h.start_commit(&a, 3).unwrap();
    assert!(h.commit(&a, -1).is_err());
    h.commit(&a, 4).unwrap();
    t.commit(&b, 5).unwrap();
    assert!(h.admitted(&a, -1).is_err());
    assert!(t.admitted(&b, 100).is_err());
    assert!(!h.admitted(&a, 100).unwrap());
    finish(&mut h, &mut t, &a, &b, true);
    assert!(t.offer(key, 22, 0, 100).is_err());
}
#[test]
fn t851_duplicate_settlement_and_retirement_cannot_release_tombstone_early() {
    let (mut h, mut t) = pair();
    let (a, b) = active(&mut h, &mut t, Resource::DisplaySession);
    assert!(h.expire(600_000).unwrap().iter().all(Option::is_none)); // Live lease has no negotiation expiry.
    h.peer_revoke(&a).unwrap();
    assert!(h.peer_revoke(&a).is_err());
    h.revoke_written(&a).unwrap();
    assert!(h.revoke_written(&a).is_err());
    t.peer_revoke(&b).unwrap();
    t.native_retired(&b).unwrap();
    assert!(t.native_retired(&b).is_err());
    assert!(!t.ready_to_ack(&b).unwrap());
    t.channels_retired(&b).unwrap();
    assert!(t.channels_retired(&b).is_err());
    assert!(t.finish(&b).is_err());
    t.acknowledge(&b).unwrap();
    assert!(t.acknowledge(&b).is_err());
    assert!(t.prepared(&b, 8).is_err());
    assert!(h.begin(Resource::DisplaySession, 1, 0, 100).is_err());
    h.acknowledge(&a).unwrap();
    assert!(h.acknowledge(&a).is_err());
    h.native_retired(&a).unwrap();
    h.channels_retired(&a).unwrap();
    h.finish(&a).unwrap();
    t.finish(&b).unwrap();
    assert!(h.finish(&a).is_err());
    assert!(h.revoke(&a).is_err());
}
#[test]
fn t851_bounded_invalid_callback_fuzz_preserves_active_resource() {
    let (mut h, mut t) = pair();
    let (a, b) = active(&mut h, &mut t, Resource::DisplaySession);
    for value in 0..4096_u64 {
        let forged = Lease {
            owner: 100 + value,
            key: Key {
                request: value,
                generation: value,
                ..a.key
            },
            ..a.clone()
        };
        assert!(!h.current(&forged));
        assert!(h.revoke(&forged).is_err());
        assert!(h.native_retired(&forged).is_err());
        assert!(h.channels_retired(&forged).is_err());
        assert!(h.peer_revoke(&forged).is_err());
        assert!(h.admitted(&forged, value as i64).is_err());
        assert!(h.finish(&forged).is_err());
        assert!(h.current(&a));
    }
    finish(&mut h, &mut t, &a, &b, true);
}
