use super::*;

#[test]
fn t1275_native_priority_and_invalid_indices_preserve_ownership() {
    let mut slots = Slots::new(2).unwrap();
    let adb = slots.reserve(Owner::Adb("sibling".into()), 0).unwrap();
    let first = blent_config::connection::pairing::PeerId::parse("a".repeat(64)).unwrap();
    slots.select(Some(first.clone()));
    assert!(!slots.adb_allowed(1));
    let native = slots.reserve(Owner::Peer(first), 1).unwrap();
    assert!(!slots.adb_allowed(1));
    assert!(slots.adb_allowed(0));
    slots.select(Some(
        blent_config::connection::pairing::PeerId::parse("b".repeat(64)).unwrap(),
    ));
    assert!(!slots.adb_allowed(0));
    for index in 2..=16 {
        assert!(!slots.adb_allowed(index));
    }
    assert!(!slots.adb_allowed(u32::MAX));
    assert_eq!(slots.closing(adb), Some(false));
    assert_eq!(slots.closing(native), Some(false));
    slots.close(adb).unwrap();
    slots.retired(adb).unwrap();
    slots.close(native).unwrap();
    slots.retired(native).unwrap();
}

#[test]
fn t1275_manual_usb_start_waits_without_evicting_another_tablet() {
    let mut slots = Slots::new(1).unwrap();
    let adb = slots.reserve(Owner::Adb("sibling".into()), 0).unwrap();
    let peer = blent_config::connection::pairing::PeerId::parse("c".repeat(64)).unwrap();
    let target = Owner::Peer(peer.clone());
    slots.set_enabled(target.clone(), false).unwrap();
    slots.set_enabled(target.clone(), true).unwrap();
    slots.select(Some(peer));
    assert_eq!(
        slots.plan(),
        Plan::Wait,
        "T1275 targeted Start preempted a sibling"
    );
    assert_eq!(slots.closing(adb), Some(false));
    slots.close(adb).unwrap();
    assert_eq!(slots.plan(), Plan::Wait);
    slots.retired(adb).unwrap();
    assert_eq!(slots.plan(), Plan::Ready(0));
    let native = slots.reserve(target, 0).unwrap();
    assert_eq!(slots.plan(), Plan::Current(native));
    slots.close(native).unwrap();
    assert_eq!(slots.plan(), Plan::Wait);
    slots.retired(native).unwrap();
}

#[test]
fn t1275_bounded_policy_matrix_preserves_siblings_and_disabled_selection() {
    for capacity in 1..=4 {
        for used in 0..=capacity {
            let mut slots = Slots::new(capacity).unwrap();
            let mut tickets = Vec::new();
            for instance in 0..used {
                tickets.push(
                    slots
                        .reserve(Owner::Adb(format!("sibling-{instance}")), instance)
                        .unwrap(),
                );
            }
            let peer = blent_config::connection::pairing::PeerId::parse("d".repeat(64)).unwrap();
            let target = Owner::Peer(peer.clone());
            slots.select(Some(peer));
            let automatic = slots.plan();
            if used == capacity {
                assert!(matches!(automatic, Plan::Replace(_)));
            } else {
                assert!(matches!(automatic, Plan::Ready(_)));
            }
            slots.set_enabled(target.clone(), false).unwrap();
            assert_eq!(
                slots.plan(),
                Plan::None,
                "T1275 disabled stale selection remained actionable"
            );
            assert_siblings(&slots, &tickets);
            slots.set_enabled(target, true).unwrap();
            if used == capacity {
                assert_eq!(slots.plan(), Plan::Wait);
            } else {
                assert!(matches!(slots.plan(), Plan::Ready(_)));
            }
            assert_siblings(&slots, &tickets);
            for ticket in tickets {
                assert_eq!(slots.closing(ticket), Some(false));
                slots.close(ticket).unwrap();
                slots.retired(ticket).unwrap();
            }
        }
    }
}

fn assert_siblings(slots: &Slots, tickets: &[Ticket]) {
    for ticket in tickets {
        assert!(
            slots.adb_allowed(ticket.instance()),
            "T1275 targeted intent displaced a sibling's eligibility"
        );
        assert_eq!(slots.closing(*ticket), Some(false));
    }
}

#[test]
fn t1274_stop_is_scoped_and_capacity_waits_for_exact_retirement() {
    let mut slots = Slots::new(2).unwrap();
    let first = Owner::Adb("first".into());
    let second = Owner::Adb("second".into());
    let a = slots.reserve(first.clone(), 0).unwrap();
    let b = slots.reserve(second.clone(), 1).unwrap();
    slots.set_enabled(first.clone(), false).unwrap();
    slots.close(a).unwrap();
    assert!(!slots.enabled(&first));
    assert!(slots.enabled(&second));
    assert_eq!(slots.closing(b), Some(false));
    assert_eq!(slots.assignments().len(), 2);
    assert_eq!(
        slots.assignments()[0].phase,
        blent_config::display_control::Phase::Stopping
    );
    assert!(slots.reserve(first.clone(), 0).is_err());
    slots.retired(a).unwrap();
    assert!(slots.reserve(first.clone(), 0).is_err());
    slots.set_enabled(first.clone(), true).unwrap();
    let replacement = slots.reserve(first, 0).unwrap();
    assert_ne!(replacement, a);
    assert!(slots.close(a).is_err());
    assert_eq!(slots.closing(replacement), Some(false));
    assert_eq!(slots.closing(b), Some(false));
}

#[test]
fn t1274_display_intent_is_bounded_and_validated() {
    let mut slots = Slots::new(1).unwrap();
    for serial in ["", "\n", &"a".repeat(1025)] {
        assert!(slots.set_enabled(Owner::Adb(serial.into()), false).is_err());
    }
    for number in 0..1024 {
        slots
            .set_enabled(Owner::Adb(format!("tablet-{number}")), false)
            .unwrap();
    }
    assert!(slots
        .set_enabled(Owner::Adb("overflow".into()), false)
        .is_err());
    slots
        .set_enabled(Owner::Adb("tablet-0".into()), false)
        .unwrap();
    slots
        .set_enabled(Owner::Adb("tablet-0".into()), true)
        .unwrap();
    slots
        .set_enabled(Owner::Adb("overflow".into()), false)
        .unwrap();
    assert!(!slots.enabled(&Owner::Adb("overflow".into())));
}
