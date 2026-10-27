use super::*;

fn id(value: &str) -> Identity {
    Identity::Adb(value.into())
}

#[test]
fn t1272_projection_exhausts_scope_capability_and_owner_states() {
    let selected = id("selected");
    let other = id("other");
    let states = [
        Activity::Unknown,
        Activity::Off,
        Activity::Starting,
        Activity::Waiting,
        Activity::InUse,
        Activity::Paused,
        Activity::Stopping,
        Activity::Failed,
    ];
    for supported in [false, true] {
        for state in states {
            for target in [None, Some(&selected), Some(&other)] {
                let view = project(
                    &selected,
                    supported,
                    Some(Observation {
                        target,
                        activity: state,
                    }),
                );
                let active = !matches!(state, Activity::Off | Activity::Failed);
                assert_eq!(view.can_stop(), active && target == Some(&selected));
                assert_eq!(view.can_start(), supported && !active);
                if active && target != Some(&selected) {
                    assert_eq!(view, ResourceView::Elsewhere(state));
                }
            }
        }
    }
}

#[test]
fn t1272_no_owner_is_not_global_proof_that_a_resource_is_off() {
    assert_eq!(project(&id("one"), true, None), ResourceView::NotStarted);
    assert_eq!(project(&id("one"), false, None), ResourceView::Unavailable);
    assert!(!ResourceView::Unavailable.can_start());
    assert!(!ResourceView::NotStarted.can_stop());
}

#[test]
fn t1272_catalog_uses_identity_and_merges_only_current_evidence() {
    let mut catalog = Catalog::new(2).unwrap();
    catalog
        .insert(Device::new(id("one"), "Same name".into(), Presence::Unknown, true).unwrap())
        .unwrap();
    catalog
        .insert(Device::new(id("two"), "Same name".into(), Presence::Available, false).unwrap())
        .unwrap();
    catalog
        .insert(
            Device::new(
                id("one"),
                "Paired tablet".into(),
                Presence::Connected,
                false,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(catalog.devices().len(), 2);
    let device = &catalog.devices()[0];
    assert_eq!(device.identity(), &id("one"));
    assert_eq!(device.name(), "Paired tablet");
    assert_eq!(device.presence(), Presence::Connected);
    assert!(device.display_assigned());
    assert!(catalog
        .insert(Device::new(id("three"), "Third".into(), Presence::Unknown, false).unwrap())
        .is_err());
    assert_eq!(catalog.into_devices().len(), 2);
    assert!(Catalog::new(2).unwrap().devices().is_empty());
}

#[test]
fn t1272_device_validation_rejects_boundaries_and_control_characters() {
    for length in [0, 1, 1023, 1024, 1025] {
        let identity = id(&"x".repeat(length));
        assert_eq!(
            Device::new(identity, "Tablet".into(), Presence::Available, false).is_ok(),
            (1..=1024).contains(&length)
        );
        assert_eq!(
            Device::new(id("valid"), "x".repeat(length), Presence::Unknown, false).is_ok(),
            length <= 1024
        );
    }
    for code in 0u8..=127 {
        let text = format!("tablet{}", char::from(code));
        let valid = !char::from(code).is_control();
        assert_eq!(
            Device::new(id(&text), "Tablet".into(), Presence::Unknown, false).is_ok(),
            valid
        );
        assert_eq!(
            Device::new(id("valid"), text, Presence::Unknown, false).is_ok(),
            valid
        );
    }
    for capacity in [0, 1, 64, 1024, 1025, usize::MAX] {
        assert_eq!(
            Catalog::new(capacity).is_ok(),
            (1..=1024).contains(&capacity)
        );
    }
}

#[test]
fn t1272_labels_and_merge_order_do_not_change_identity_or_evidence() {
    let peer = crate::connection::pairing::PeerId::parse("ab".repeat(32)).unwrap();
    let device = Device::new(
        Identity::UsbPeer(peer.clone()),
        String::new(),
        Presence::Connected,
        false,
    )
    .unwrap();
    assert_eq!(device.name(), "USB tablet (abababababab)");
    assert_eq!(device.identity(), &Identity::UsbPeer(peer));
    let observations = [
        Device::new(id("one"), "Zulu".into(), Presence::Connected, false).unwrap(),
        Device::new(id("one"), "Alpha".into(), Presence::Connected, true).unwrap(),
        Device::new(id("one"), "Earlier".into(), Presence::Available, false).unwrap(),
    ];
    let mut forward = Catalog::new(1).unwrap();
    let mut reverse = Catalog::new(1).unwrap();
    for device in observations.iter().cloned() {
        forward.insert(device).unwrap();
    }
    for device in observations.into_iter().rev() {
        reverse.insert(device).unwrap();
    }
    assert_eq!(forward.devices(), reverse.devices());
    assert_eq!(forward.devices()[0].name(), "Alpha");
}
