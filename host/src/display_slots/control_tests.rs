use super::*;

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
