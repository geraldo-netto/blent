use super::*;

#[test]
fn t853_invalid_capacities_and_duplicate_inventory_do_not_mutate() {
    for bad in [0, 513, usize::MAX] {
        assert!(Control::new(
            peer(0),
            vec![],
            Limits {
                devices: bad,
                ..Limits::default()
            }
        )
        .is_err());
    }
    for bad in [0, 65, usize::MAX] {
        assert!(Control::new(
            peer(0),
            vec![],
            Limits {
                links: bad,
                ..Limits::default()
            }
        )
        .is_err());
    }
    for bad in [0, 1025, usize::MAX] {
        assert!(Control::new(
            peer(0),
            vec![],
            Limits {
                clients: bad,
                ..Limits::default()
            }
        )
        .is_err());
    }
    assert!(Control::new(peer(0), vec![peer(1), peer(1)], Limits::default()).is_err());
    assert!(Control::new(peer(0), vec![peer(1); 1025], Limits::default()).is_err());
    let mut rig = Rig::new(vec![]);
    let before = rig.control.snapshot();
    assert!(rig
        .control
        .replace_devices(vec![device(1), device(1)])
        .is_err());
    assert!(rig
        .control
        .replace_devices((0..65).map(device).collect())
        .is_err());
    assert_eq!(rig.control.snapshot(), before);
}

#[test]
fn t853_timeout_boundaries_and_overflows_reject_before_mutation() {
    for bad in [-1, 0, 60_001, i64::MAX] {
        assert!(Control::new(
            peer(0),
            vec![],
            Limits {
                setup_ms: bad,
                ..Limits::default()
            }
        )
        .is_err());
    }
    for bad in [-1, 0, 600_001, i64::MAX] {
        assert!(Control::new(
            peer(0),
            vec![],
            Limits {
                approval_ms: bad,
                ..Limits::default()
            }
        )
        .is_err());
    }
    let mut rig = Rig::new(vec![]);
    let target = *rig.control.devices.keys().next().unwrap();
    for now in [-1, i64::MAX] {
        assert!(rig
            .control
            .command(
                rig.client,
                Command::Connect {
                    revision: rig.control.revision,
                    target
                },
                now
            )
            .is_err());
    }
    assert!(rig.control.tick(-1).is_err());
    let id = rig.start(1);
    assert!(rig.control.tick(59_999).unwrap().is_empty());
    assert_eq!(rig.control.tick(60_000).unwrap(), vec![id]);
    assert!(rig.control.tick(i64::MAX).unwrap().is_empty());
    rig.control.start_failed(id).unwrap();
    assert_eq!(rig.control.collect_retired().unwrap(), vec![id]);
}

#[test]
fn t853_all_admission_capacities_and_identifier_exhaustion_remain_bounded() {
    let mut rig = Rig::new(vec![]);
    rig.control.limits.clients = 1;
    assert!(rig.control.admit_client().is_err());
    rig.control.limits.links = 1;
    let id = rig.start(1);
    let target = *rig
        .control
        .devices
        .iter()
        .find(|(_, value)| value.key == 2)
        .unwrap()
        .0;
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Connect {
                revision: rig.control.revision,
                target
            },
            0
        )
        .is_err());
    rig.control.start_failed(id).unwrap();
    rig.control.collect_retired().unwrap();
    rig.control.next = MAX_COUNTER;
    rig.control.drop_client(rig.client);
    assert_eq!(rig.control.admit_client().unwrap(), MAX_COUNTER);
    rig.control.drop_client(MAX_COUNTER);
    assert!(rig.control.admit_client().is_err());
    assert!(rig.control.replace_devices(vec![device(1)]).is_err());
    rig.control.revision = MAX_COUNTER;
    assert!(rig.control.replace_devices(vec![]).is_err());
    assert!(rig.control.tick(0).is_err());
    assert!(rig.control.changed().is_err());
}

#[test]
fn t853_unknown_ids_and_invalid_commands_leave_snapshot_unchanged() {
    let mut rig = Rig::new(vec![]);
    let before = rig.control.snapshot();
    for id in [0, 999, MAX_COUNTER, u64::MAX] {
        reject_unknown(&mut rig, id);
    }
    assert_eq!(rig.control.snapshot(), before);
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Decide {
                connection: 1,
                ticket: Ticket {
                    epoch: 1,
                    peer: peer(1)
                },
                allow: false,
                remember: true
            },
            0
        )
        .is_err());
}
fn reject_unknown(rig: &mut Rig, id: u64) {
    let hold = rig.pool.reserve().unwrap();
    assert!(rig.control.attach(id, hold.completion()).is_err());
    assert!(rig.control.start_failed(id).is_err());
    assert!(rig.control.failed(id).is_err());
    assert!(rig.control.quarantine(id).is_err());
    assert!(rig.control.native_finished(id).is_err());
    assert!(rig.control.pairing(id).is_err());
    assert!(rig.control.lease(id).is_none());
    assert!(rig.control.identified(id, peer(1), 0).is_err());
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Disconnect {
                connection: id,
                ticket: None
            },
            0
        )
        .is_err());
}

#[test]
fn t853_bounded_stale_ticket_fuzz_never_changes_current_approval() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, _hold) = rig.pending(1, peer(1));
    let before = rig.control.snapshot();
    let mut value = 0x617a6a43_u64;
    for _ in 0..4096 {
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        let stale = Ticket {
            epoch: value.max(2),
            peer: ticket.peer.clone(),
        };
        assert!(rig
            .control
            .command(
                rig.client,
                Command::Decide {
                    connection: id,
                    ticket: stale,
                    allow: true,
                    remember: false
                },
                2
            )
            .is_err());
    }
    assert_eq!(rig.control.snapshot(), before);
}

#[test]
fn t853_late_proof_and_authority_callbacks_cannot_replace_current_ticket() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, _hold) = rig.pending(1, peer(1));
    rig.approve(id, &ticket, false);
    assert!(rig
        .control
        .prepare_authentication(id, &ticket, peer(1), 120_001)
        .is_err());
    let mut reply = rig.job(id, &ticket).run(&mut rig.trust);
    reply.pending.id += 1;
    assert!(rig.control.complete_authority(reply, 4).is_err());
    assert!(rig.control.lease(id).is_none());
    assert!(rig.control.pairing(id).is_err());
    assert!(rig.control.collect_retired().unwrap().is_empty());
}
