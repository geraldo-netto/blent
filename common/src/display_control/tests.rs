use super::*;

fn snapshot() -> Snapshot {
    Snapshot {
        owner: "a".repeat(64),
        assignments: vec![Assignment {
            target: Target::Adb("tablet".into()),
            lease: Lease {
                slot: 0,
                generation: 1,
            },
            phase: Phase::Assigned,
        }],
    }
}

#[test]
fn t1272_display_contract_round_trips_without_backend_types() {
    let mut snapshot = snapshot();
    snapshot.assignments.push(Assignment {
        target: Target::Usb("b".repeat(64)),
        lease: Lease {
            slot: 1,
            generation: 2,
        },
        phase: Phase::Stopping,
    });
    snapshot.validate().unwrap();
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    let decoded: Snapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot, decoded);
    assert_eq!(
        Target::from_identity(&Identity::Adb("x".into())),
        Target::Adb("x".into())
    );
    let peer = Identity::UsbPeer(PeerId::parse("b".repeat(64)).unwrap());
    assert_eq!(Target::from_identity(&peer).identity().unwrap(), peer);
    Command::Inspect {}.validate().unwrap();
    for enabled in [false, true] {
        let command = Command::Set {
            owner: snapshot.owner.clone(),
            target: Target::Adb("tablet".into()),
            enabled,
            expected: (!enabled).then_some(Lease {
                slot: 0,
                generation: 1,
            }),
        };
        command.validate().unwrap();
        let encoded = serde_json::to_vec(&command).unwrap();
        assert_eq!(
            serde_json::from_slice::<Command>(&encoded).unwrap(),
            command
        );
    }
}

#[test]
fn t1272_display_contract_rejects_malformed_and_stale_lease_shapes() {
    for slot in [0, 3, 4, u32::MAX] {
        for generation in [0, 1, u64::MAX] {
            assert_eq!(
                Lease { slot, generation }.validate().is_ok(),
                slot < 4 && generation != 0
            );
        }
    }
    for target in [
        Target::Adb(String::new()),
        Target::Adb("x".repeat(1025)),
        Target::Adb("a\nb".into()),
        Target::Usb("x".repeat(64)),
        Target::Usb("b".repeat(63)),
    ] {
        assert!(target.identity().is_err());
    }
    let valid = snapshot();
    for enabled in [false, true] {
        let command = Command::Set {
            owner: valid.owner.clone(),
            target: Target::Adb("tablet".into()),
            enabled,
            expected: enabled.then_some(Lease {
                slot: 0,
                generation: 1,
            }),
        };
        assert!(command.validate().is_err());
    }
    assert!(serde_json::from_str::<Command>(r#"{"command":"inspect","unexpected":true}"#).is_err());
}

#[test]
fn t1272_display_snapshots_reject_ambiguous_targets_and_unbounded_inputs() {
    let original = snapshot();
    let mut duplicate = original.clone();
    duplicate.assignments.push(duplicate.assignments[0].clone());
    assert!(duplicate.validate().is_err());
    duplicate.assignments[1].target = Target::Adb("other".into());
    assert!(duplicate.validate().is_err());
    let mut oversized = original.clone();
    oversized.assignments = vec![oversized.assignments[0].clone(); 5];
    assert!(oversized.validate().is_err());
    for length in [0, 63, 65, 1024] {
        let mut invalid = original.clone();
        invalid.owner = "a".repeat(length);
        assert!(invalid.validate().is_err());
        assert!(Command::Set {
            owner: invalid.owner,
            target: Target::Adb("x".into()),
            enabled: true,
            expected: None
        }
        .validate()
        .is_err());
    }
}
