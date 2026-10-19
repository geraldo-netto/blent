use super::*;
use crate::connection::{
    management::Resource,
    pairing::{PeerId, Ticket},
    resources::{Resources, Role},
};

fn resources() -> Resources {
    Resources::new(
        Role::Host,
        Ticket {
            epoch: 1,
            peer: PeerId::parse("ab".repeat(32)).unwrap(),
        },
    )
    .unwrap()
}
#[test]
fn t1103_typed_admission_timeout_bounds_and_seeded_invalid_values() {
    assert_eq!(AdmissionTimeout::default().milliseconds(), 120_000);
    for value in [1, 37, 120_000, 600_000] {
        assert_eq!(AdmissionTimeout::new(value).unwrap().milliseconds(), value);
    }
    for value in [i64::MIN, -1, 0, 600_001, i64::MAX] {
        assert!(AdmissionTimeout::new(value).is_err());
    }
    let mut seed = 1103u64;
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = (seed % 1_200_001) as i64 - 300_000;
        assert_eq!(
            AdmissionTimeout::new(value).is_ok(),
            (1..=600_000).contains(&value)
        );
    }
}
#[test]
fn t1103_invalid_and_overflowing_deadlines_publish_no_resource() {
    for (now, timeout) in [
        (0, 0),
        (0, -1),
        (0, 600_001),
        (0, i64::MAX),
        (-1, 37),
        (i64::MAX, 1),
        (i64::MAX - 36, 37),
    ] {
        let mut resources = resources();
        assert!(resources
            .begin(Resource::DisplaySession, 1, now, timeout)
            .is_err());
        let lease = resources
            .begin(Resource::DisplaySession, 1, 10, 37)
            .unwrap();
        assert_eq!(lease.key.request, 1);
        assert_eq!(resources.expire(46).unwrap(), [None, None, None, None]);
        assert_eq!(
            resources.expire(47).unwrap(),
            [Some(lease), None, None, None]
        );
    }
}

#[test]
fn t1103_late_peer_preparation_cannot_grant_expired_resource() {
    for milliseconds in [1, 37, 600_000] {
        let mut before = resources();
        let lease = before
            .begin(Resource::DisplaySession, 1, 10, milliseconds)
            .unwrap();
        assert!(before.prepared(&lease, 10 + milliseconds - 1).unwrap());
        assert!(before.start_commit(&lease, 10 + milliseconds).is_err());
        assert!(!before.current(&lease));
        let mut late = resources();
        let lease = late
            .begin(Resource::DisplaySession, 1, 10, milliseconds)
            .unwrap();
        assert!(!late.prepared(&lease, 10 + milliseconds).unwrap());
        assert!(late.start_commit(&lease, 10 + milliseconds).is_err());
        assert!(!late.current(&lease));
    }
}

fn tablet() -> Resources {
    Resources::new(
        Role::Tablet,
        Ticket {
            epoch: 1,
            peer: PeerId::parse("ab".repeat(32)).unwrap(),
        },
    )
    .unwrap()
}
fn key() -> crate::connection::management::Key {
    crate::connection::management::Key {
        resource: Resource::DisplaySession,
        request: 1,
        generation: 1,
    }
}
#[test]
fn t1103_tablet_offers_preserve_selected_timeout_boundaries() {
    for milliseconds in [1, 37, 120_000, 600_000] {
        let mut tablet = tablet();
        let lease = tablet.offer(key(), 17, 10, milliseconds).unwrap();
        assert_eq!(
            tablet.expire(10 + milliseconds - 1).unwrap(),
            [None, None, None, None]
        );
        assert_eq!(
            tablet.expire(10 + milliseconds).unwrap(),
            [Some(lease), None, None, None]
        );
    }
    let mut host = resources();
    assert!(host.offer(key(), 17, 0, 37).is_err());
    assert_eq!(
        host.begin(Resource::DisplaySession, 17, 0, 37)
            .unwrap()
            .key
            .request,
        1
    );
}
#[test]
fn t1103_invalid_tablet_offers_reject_without_consuming_request_identity() {
    for (now, timeout) in [
        (0, 0),
        (0, -1),
        (0, 600_001),
        (0, i64::MAX),
        (-1, 37),
        (i64::MAX, 1),
    ] {
        let mut tablet = tablet();
        assert!(tablet.offer(key(), 17, now, timeout).is_err());
        let lease = tablet.offer(key(), 17, 0, 1).unwrap();
        assert_eq!(lease.key, key());
        assert_eq!(tablet.expire(0).unwrap(), [None, None, None, None]);
        assert_eq!(tablet.expire(1).unwrap(), [Some(lease), None, None, None]);
    }
    let mut tablet = tablet();
    let mut invalid = key();
    invalid.request = 0;
    assert!(tablet.offer(invalid, 17, 0, 37).is_err());
    assert!(tablet.offer(key(), 0, 0, 37).is_err());
    assert!(tablet.offer(key(), 17, 0, 37).is_ok());
}
