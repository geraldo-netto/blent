use super::*;

#[test]
fn t1596_unchanged_scan_preserves_targets_and_pending_connect() {
    let mut rig = Rig::new(vec![]);
    let snapshot = rig.control.snapshot();
    let revision = rig.control.revision;
    let target = *rig.control.devices.keys().next().unwrap();
    let next = rig.control.next;
    rig.control
        .replace_devices((1..=4).rev().map(device).collect())
        .unwrap();
    assert_eq!(
        rig.control.snapshot(),
        snapshot,
        "T1596 unchanged inventory invalidated UI"
    );
    assert_eq!(rig.control.next, next);
    assert!(matches!(
        rig.control
            .command(rig.client, Command::Connect { revision, target }, 0)
            .unwrap(),
        Action::Start { target: 1, .. }
    ));
}

#[test]
fn t1596_metadata_changes_keep_identity_but_reject_stale_revision() {
    use blent_config::connection::control::{Availability, DeviceNames};
    for field in 0..5 {
        let mut rig = Rig::new(vec![]);
        rig.control.replace_devices(vec![device(1)]).unwrap();
        let target = *rig.control.devices.keys().next().unwrap();
        let revision = rig.control.revision;
        let mut changed = device(1);
        match field {
            0 => changed.vendor += 1,
            1 => changed.product += 1,
            2 => changed.accessory = false,
            3 => changed.availability = Availability::Debugging,
            _ => changed.names = DeviceNames::new("maker".into(), "model".into()).unwrap(),
        }
        rig.control.replace_devices(vec![changed]).unwrap();
        assert_eq!(*rig.control.devices.keys().next().unwrap(), target);
        assert_eq!(rig.control.revision, revision + 1);
        assert!(rig
            .control
            .command(rig.client, Command::Connect { revision, target }, 0)
            .is_err());
    }
}

#[test]
fn t1596_removed_identity_is_never_reused_and_payload_is_refreshed() {
    let mut rig = Rig::new(vec![]);
    rig.control.replace_devices(vec![device(1)]).unwrap();
    let original = *rig.control.devices.keys().next().unwrap();
    let revision = rig.control.revision;
    let mut fresh = device(1);
    fresh.payload = 100;
    rig.control.replace_devices(vec![fresh]).unwrap();
    assert_eq!(rig.control.revision, revision);
    assert_eq!(rig.control.devices[&original].payload, 100);
    rig.control.replace_devices(vec![]).unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    assert!(!rig.control.devices.contains_key(&original));
}

#[test]
fn t1596_exhaustion_rejects_atomically_without_blocking_unchanged_inventory() {
    let mut rig = Rig::new(vec![]);
    let before = rig.control.snapshot();
    rig.control.next = MAX_COUNTER;
    assert!(rig
        .control
        .replace_devices(vec![device(5), device(6)])
        .is_err());
    assert_eq!(rig.control.snapshot(), before);
    assert_eq!(rig.control.next, MAX_COUNTER);
    rig.control.next = MAX_COUNTER + 1;
    rig.control
        .replace_devices((1..=4).map(device).collect())
        .unwrap();
    assert_eq!(rig.control.snapshot(), before);
    rig.control.revision = MAX_COUNTER;
    let before = rig.control.snapshot();
    rig.control
        .replace_devices((1..=4).map(device).collect())
        .unwrap();
    assert!(rig.control.replace_devices(vec![]).is_err());
    assert_eq!(rig.control.snapshot(), before);
}
