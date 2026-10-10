use super::*;
fn peer(byte: u8) -> PeerId {
    PeerId::parse(format!("{byte:02x}").repeat(32)).unwrap()
}
fn adb(serial: &str) -> Owner {
    Owner::Adb(serial.into())
}
fn retire(slots: &mut Slots, ticket: Ticket) {
    slots.close(ticket).unwrap();
    slots.retired(ticket).unwrap();
}
#[test]
fn t917_selected_usb_uses_free_capacity_then_replaces_one_adb() {
    let mut slots = Slots::new(2).unwrap();
    assert_eq!(slots.plan(), Plan::None);
    let primary = slots.reserve(adb("first"), 0).unwrap();
    slots.select(Some(peer(1)));
    assert_eq!(slots.plan(), Plan::Ready(1));
    assert_eq!(slots.next_adb_extra(), None);
    assert!(slots.reserve(adb("second"), 1).is_err());
    let native = slots.reserve(Owner::Peer(peer(1)), 1).unwrap();
    assert_eq!(slots.plan(), Plan::Current(native));
    assert_eq!(slots.owner(primary), Some(&adb("first")));
    slots.select(Some(peer(2)));
    assert_eq!(slots.plan(), Plan::Replace(primary));
    slots.close(primary).unwrap();
    slots.close(primary).unwrap();
    assert_eq!(slots.plan(), Plan::Wait);
    assert!(slots.reserve(Owner::Peer(peer(2)), 0).is_err());
    assert!(slots.retired(native).is_err());
    slots.retired(primary).unwrap();
    let replacement = slots.reserve(Owner::Peer(peer(2)), 0).unwrap();
    assert_ne!(primary, replacement);
    assert!(slots.close(primary).is_err());
    assert!(slots.retired(primary).is_err());
    assert_eq!(slots.owner(primary), None);
    slots.select(Some(peer(3)));
    assert_eq!(slots.plan(), Plan::Wait); // Never displace another native peer.
    retire(&mut slots, native);
    assert_eq!(slots.plan(), Plan::Ready(1));
}
#[test]
fn t917_keeps_primary_and_waits_for_existing_retirement() {
    let mut slots = Slots::new(3).unwrap();
    let first = slots.reserve(adb("first"), 0).unwrap();
    let second = slots.reserve(adb("second"), 1).unwrap();
    let third = slots.reserve(adb("third"), 2).unwrap();
    slots.select(Some(peer(1)));
    assert_eq!(slots.plan(), Plan::Replace(third));
    slots.close(second).unwrap();
    assert_eq!(slots.plan(), Plan::Wait);
    slots.retired(second).unwrap();
    assert_eq!(slots.plan(), Plan::Ready(1));
    assert_eq!(slots.owner(first), Some(&adb("first")));
    assert_eq!(slots.owner(third), Some(&adb("third")));
}
#[test]
fn t917_selection_changes_do_not_release_native_ownership() {
    let mut slots = Slots::new(1).unwrap();
    slots.select(Some(peer(1)));
    let old = slots.reserve(Owner::Peer(peer(1)), 0).unwrap();
    slots.select(None);
    assert_eq!(slots.plan(), Plan::None);
    assert!(!slots.adb_available(0));
    slots.select(Some(peer(2)));
    assert_eq!(slots.plan(), Plan::Wait);
    slots.close(old).unwrap();
    slots.select(Some(peer(1)));
    assert_eq!(slots.plan(), Plan::Wait);
    slots.retired(old).unwrap();
    let new = slots.reserve(Owner::Peer(peer(1)), 0).unwrap();
    assert!(slots.retired(old).is_err());
    assert_eq!(slots.plan(), Plan::Current(new));
    retire(&mut slots, new);
    slots.select(None);
    assert!(slots.adb_available(0));
}
#[test]
fn t917_bounded_invalid_inputs_and_identity_separation() {
    for capacity in [0, 5, 255, u32::MAX] {
        assert!(Slots::new(capacity).is_err());
    }
    for capacity in 1..=4 {
        let mut slots = Slots::new(capacity).unwrap();
        for instance in [capacity, 255, u32::MAX] {
            assert!(!slots.adb_available(instance));
            assert!(slots.reserve(adb("valid"), instance).is_err());
            let invalid = Ticket {
                instance,
                revision: 1,
            };
            assert_eq!(slots.owner(invalid), None);
            assert!(slots.close(invalid).is_err());
            assert!(slots.retired(invalid).is_err());
        }
        for byte in 0..=u8::MAX {
            let invalid = format!("serial{}", char::from(byte));
            if char::from(byte).is_control() {
                assert!(slots.reserve(adb(&invalid), 0).is_err());
            }
        }
        for invalid in [String::new(), "x".repeat(1025)] {
            assert!(slots.reserve(adb(&invalid), 0).is_err());
        }
        let valid = slots.reserve(adb(&"x".repeat(1024)), 0).unwrap();
        assert!(slots.reserve(adb(&"x".repeat(1024)), capacity - 1).is_err());
        retire(&mut slots, valid);
        assert!(slots.reserve(Owner::Peer(peer(1)), 0).is_err());
        slots.select(Some(peer(1)));
        assert!(slots.reserve(Owner::Peer(peer(2)), 0).is_err());
        slots.select(None);
        let alias = slots.reserve(adb(peer(1).as_str()), 0).unwrap();
        slots.select(Some(peer(1)));
        assert_ne!(slots.plan(), Plan::Current(alias));
    }
}
#[test]
fn t917_revision_exhaustion_never_wraps() {
    let mut slots = Slots::new(2).unwrap();
    slots.revision = u64::MAX - 1;
    let last = slots.reserve(adb("last"), 0).unwrap();
    assert_eq!(last.instance(), 0);
    assert_eq!(last.revision, u64::MAX);
    assert!(slots.reserve(adb("overflow"), 1).is_err());
    retire(&mut slots, last);
    assert!(slots.reserve(adb("last"), 0).is_err());
    assert_eq!(slots.next_adb_extra(), Some(1));
}

#[test]
fn t917_exhaustion_preserves_working_display_instead_of_preempting() {
    let mut slots = Slots::new(1).unwrap();
    slots.revision = u64::MAX - 1;
    let working = slots.reserve(adb("working"), 0).unwrap();
    slots.select(Some(peer(1)));
    assert_eq!(
        slots.plan(),
        Plan::Wait,
        "T917 exhausted owner cannot admit a replacement"
    );
    assert_eq!(slots.owner(working), Some(&adb("working")));
    assert!(slots.adb_allowed(0));
}
