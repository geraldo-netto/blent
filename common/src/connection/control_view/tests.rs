use super::super::wire::MAX_COUNTER;
use super::*;

fn peer(value: u64) -> PeerId {
    PeerId::parse(format!("{value:064x}")).unwrap()
}
fn ticket() -> Ticket {
    Ticket {
        epoch: 3,
        peer: peer(2),
    }
}
fn begin(revision: u64) -> Notice {
    Notice::Begin {
        revision,
        local: peer(1),
    }
}
fn device(revision: u64, target: u64) -> Notice {
    Notice::Device {
        revision,
        target,
        vendor: 0x18d1,
        product: 0x2d00,
        accessory: true,
    }
}
fn link(revision: u64, connection: u64, state: State) -> Notice {
    Notice::Link {
        revision,
        connection,
        ticket: Some(ticket()),
        state,
        trusted: false,
    }
}
fn snapshot(revision: u64, state: State) -> Snapshot {
    let mut builder = Assembler::new(Limits::default()).unwrap();
    for value in [
        begin(revision),
        device(revision, 1),
        link(revision, 2, state),
        Notice::Saved {
            revision,
            peer: peer(4),
        },
    ] {
        assert!(builder.push(value).unwrap().is_none());
    }
    builder.push(Notice::End { revision }).unwrap().unwrap()
}
fn decide() -> Command {
    Command::Decide {
        connection: 2,
        ticket: ticket(),
        allow: true,
        remember: true,
    }
}
fn disconnect() -> Command {
    Command::Disconnect {
        connection: 2,
        ticket: Some(ticket()),
    }
}

#[test]
fn t900_complete_snapshot_preserves_corresponding_identity_and_sorted_metadata() {
    let snapshot = snapshot(5, State::Pending);
    assert_eq!(snapshot.revision(), 5);
    assert_eq!(snapshot.computer(), &peer(1));
    assert_eq!(
        snapshot.devices()[&1],
        Device {
            vendor: 0x18d1,
            product: 0x2d00,
            accessory: true
        }
    );
    assert_eq!(snapshot.links()[&2].ticket, Some(ticket()));
    assert!(!snapshot.links()[&2].trusted);
    assert_eq!(snapshot.saved()[peer(4).as_str()], peer(4));
}

#[test]
fn t900_invalid_order_revision_and_duplicate_records_discard_partial_state() {
    let mut builder = Assembler::new(Limits::default()).unwrap();
    assert!(builder.push(Notice::End { revision: 1 }).is_err());
    builder.push(begin(1)).unwrap();
    assert!(builder.push(begin(1)).is_err());
    assert!(builder.push(Notice::End { revision: 1 }).is_err());
    builder.push(begin(1)).unwrap();
    assert!(builder.push(device(2, 1)).is_err());
    assert!(builder.push(Notice::End { revision: 1 }).is_err());
    for notice in [
        device(1, 1),
        link(1, 2, State::Pending),
        Notice::Saved {
            revision: 1,
            peer: peer(4),
        },
    ] {
        builder.push(begin(1)).unwrap();
        builder.push(notice.clone()).unwrap();
        assert!(builder.push(notice).is_err());
        assert!(builder.push(Notice::End { revision: 1 }).is_err());
    }
    builder.push(begin(9)).unwrap();
    assert!(builder.push(Notice::End { revision: 9 }).unwrap().is_some());
}

#[test]
fn t900_snapshot_capacities_reject_overflow_without_publishing_partial_lists() {
    let limits = Limits {
        devices: 1,
        links: 1,
        saved: 1,
    };
    let pairs = [
        (device(1, 1), device(1, 2)),
        (link(1, 1, State::Pending), link(1, 2, State::Pending)),
        (
            Notice::Saved {
                revision: 1,
                peer: peer(4),
            },
            Notice::Saved {
                revision: 1,
                peer: peer(5),
            },
        ),
    ];
    for (first, extra) in pairs {
        let mut builder = Assembler::new(limits).unwrap();
        builder.push(begin(1)).unwrap();
        builder.push(first).unwrap();
        assert!(builder.push(extra).is_err());
        assert!(builder.push(Notice::End { revision: 1 }).is_err());
    }
}

#[test]
fn t900_all_advertised_capacity_maxima_fit_one_bounded_snapshot() {
    let limits = Limits::default();
    let mut builder = Assembler::new(limits).unwrap();
    builder.push(begin(MAX_COUNTER)).unwrap();
    for id in 1..=limits.devices {
        builder.push(device(MAX_COUNTER, id as u64)).unwrap();
    }
    for id in 1..=limits.links {
        builder
            .push(link(MAX_COUNTER, id as u64, State::Pending))
            .unwrap();
    }
    for id in 1..=limits.saved {
        builder
            .push(Notice::Saved {
                revision: MAX_COUNTER,
                peer: peer(id as u64),
            })
            .unwrap();
    }
    let view = builder
        .push(Notice::End {
            revision: MAX_COUNTER,
        })
        .unwrap()
        .unwrap();
    assert_eq!(view.devices().len(), limits.devices);
    assert_eq!(view.links().len(), limits.links);
    assert_eq!(view.saved().len(), limits.saved);
}

#[test]
fn t900_capacity_and_counter_boundaries_are_rejected_by_the_shared_contract() {
    for invalid in [0, 1025, usize::MAX] {
        assert!(Assembler::new(Limits {
            devices: invalid,
            ..Limits::default()
        })
        .is_err());
        assert!(Assembler::new(Limits {
            links: invalid,
            ..Limits::default()
        })
        .is_err());
        assert!(Assembler::new(Limits {
            saved: invalid,
            ..Limits::default()
        })
        .is_err());
    }
    assert!(Assembler::new(Limits {
        devices: 513,
        ..Limits::default()
    })
    .is_err());
    assert!(Assembler::new(Limits {
        links: 65,
        ..Limits::default()
    })
    .is_err());
    let mut builder = Assembler::new(Limits::default()).unwrap();
    for counter in [0, MAX_COUNTER + 1, u64::MAX] {
        assert!(builder.push(begin(counter)).is_err());
        builder.push(begin(1)).unwrap();
        assert!(builder.push(device(1, counter)).is_err());
        assert!(builder.push(Notice::End { revision: 1 }).is_err());
    }
}

#[test]
fn t900_identified_or_trusted_connections_require_a_displayed_peer_ticket() {
    for state in [
        State::Discovering,
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ] {
        for trusted in [false, true] {
            let mut builder = Assembler::new(Limits::default()).unwrap();
            builder.push(begin(1)).unwrap();
            let result = builder.push(Notice::Link {
                revision: 1,
                connection: 1,
                ticket: None,
                state,
                trusted,
            });
            let allowed = !trusted
                && matches!(
                    state,
                    State::Discovering | State::Closing | State::Quarantined
                );
            assert_eq!(result.is_ok(), allowed);
        }
    }
}

#[test]
fn t900_displayed_actions_are_revalidated_without_optimistic_trust_or_approval() {
    let mut model = Model::default();
    assert!(model.view().is_none());
    let view = model.publish(snapshot(1, State::Pending)).unwrap();
    assert_eq!(model.view().unwrap().snapshot(), view.snapshot());
    let commands = [
        Command::Inspect,
        Command::Connect {
            revision: 1,
            target: 1,
        },
        decide(),
        disconnect(),
        Command::Revoke { peer: peer(2) },
        Command::Revoke { peer: peer(4) },
    ];
    for command in commands {
        let action = view.prepare(command.clone()).unwrap();
        assert_eq!(model.command(&action).unwrap(), &command);
        assert_eq!(
            model.view().unwrap().snapshot().links()[&2].state,
            State::Pending
        );
    }
    model.publish(snapshot(1, State::Pending)).unwrap();
    let action = view.prepare(decide()).unwrap();
    model.publish(snapshot(2, State::Pending)).unwrap();
    assert!(model.command(&action).is_ok());
    model.publish(snapshot(3, State::Approved)).unwrap();
    assert!(model.command(&action).is_err());
    assert!(model.command(&view.prepare(disconnect()).unwrap()).is_ok());
    model.publish(snapshot(4, State::Closing)).unwrap();
    assert!(model.command(&view.prepare(disconnect()).unwrap()).is_err());
}

#[test]
fn t900_replacement_broker_cannot_reuse_any_old_approval_even_with_equal_counters() {
    let mut model = Model::default();
    let view = model
        .publish(snapshot(MAX_COUNTER, State::Pending))
        .unwrap();
    let action = view.prepare(decide()).unwrap();
    model.reset();
    assert!(model.view().is_none());
    assert!(model.command(&action).is_err());
    model
        .publish(snapshot(MAX_COUNTER, State::Pending))
        .unwrap();
    assert!(model.command(&action).is_err());
    let current = model.view().unwrap().prepare(decide()).unwrap();
    assert!(model.command(&current).is_ok());
}

#[test]
fn t900_conflicting_or_backward_snapshots_invalidate_all_displayed_actions() {
    let mut model = Model::default();
    for bad in [
        snapshot(1, State::Pending),
        snapshot(2, State::Approved),
        changed_computer(),
    ] {
        let view = model.publish(snapshot(2, State::Pending)).unwrap();
        let action = view.prepare(decide()).unwrap();
        assert!(model.publish(bad).is_err());
        assert!(model.view().is_none());
        assert!(model.command(&action).is_err());
    }
}
fn changed_computer() -> Snapshot {
    let mut builder = Assembler::new(Limits::default()).unwrap();
    builder
        .push(Notice::Begin {
            revision: 3,
            local: peer(99),
        })
        .unwrap();
    builder.push(Notice::End { revision: 3 }).unwrap().unwrap()
}

#[test]
fn t900_unknown_mutated_and_stale_choices_never_become_control_commands() {
    let mut model = Model::default();
    let view = model.publish(snapshot(1, State::Pending)).unwrap();
    let choices = [
        Command::Connect {
            revision: 2,
            target: 1,
        },
        Command::Connect {
            revision: 1,
            target: 99,
        },
        Command::Decide {
            connection: 2,
            ticket: Ticket {
                epoch: 4,
                peer: peer(2),
            },
            allow: true,
            remember: false,
        },
        Command::Decide {
            connection: 2,
            ticket: ticket(),
            allow: false,
            remember: true,
        },
        Command::Disconnect {
            connection: 2,
            ticket: None,
        },
        Command::Disconnect {
            connection: 99,
            ticket: None,
        },
        Command::Revoke { peer: peer(99) },
    ];
    for command in choices {
        assert!(view.prepare(command).is_err());
    }
    let action = view
        .prepare(Command::Connect {
            revision: 1,
            target: 1,
        })
        .unwrap();
    model.publish(snapshot(2, State::Pending)).unwrap();
    assert!(model.command(&action).is_err());
    let mut forged = view.prepare(decide()).unwrap();
    forged.computer = peer(99);
    assert!(model.command(&forged).is_err());
    model.current = None;
    assert!(model
        .command(&view.prepare(Command::Inspect).unwrap())
        .is_err());
}

#[test]
fn t900_seeded_invalid_sequences_remain_bounded_and_only_complete_on_end() {
    let mut seed = 900_u64;
    let mut builder = Assembler::new(Limits {
        devices: 2,
        links: 2,
        saved: 2,
    })
    .unwrap();
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let revision = (seed >> 8) % 4;
        let notice = match seed % 5 {
            0 => begin(revision),
            1 => device(revision, seed % 8),
            2 => link(revision, seed % 8, State::Pending),
            3 => Notice::Saved {
                revision,
                peer: peer(seed % 8),
            },
            _ => Notice::End { revision },
        };
        let end = matches!(notice, Notice::End { .. });
        if let Ok(Some(snapshot)) = builder.push(notice) {
            assert!(end);
            assert_eq!(snapshot.revision(), revision);
            assert!(snapshot.devices().len() <= 2);
            assert!(snapshot.links().len() <= 2);
            assert!(snapshot.saved().len() <= 2);
        }
    }
}

#[test]
fn t924_equal_broker_counters_do_not_identify_a_lifetime() {
    let mut model = Model::default();
    let original = model.publish(snapshot(1, State::Authenticated)).unwrap();
    let refresh = model.publish(snapshot(2, State::Authenticated)).unwrap();
    assert!(original.same_lifetime(&refresh));
    assert!(original.same_lifetime(&original.clone()));
    model.reset();
    let replacement = model.publish(snapshot(1, State::Authenticated)).unwrap();
    assert!(!original.same_lifetime(&replacement));
    let independent = Model::default()
        .publish(snapshot(1, State::Authenticated))
        .unwrap();
    assert!(!replacement.same_lifetime(&independent));
}

#[test]
fn t924_lifetime_tokens_do_not_pin_obsolete_snapshots() {
    let mut model = Model::default();
    let view = model.publish(snapshot(1, State::Authenticated)).unwrap();
    let weak = Arc::downgrade(&view.snapshot);
    let lifetime = view.lifetime();
    assert!(lifetime.same_as(&view.lifetime()));
    drop(view);
    model.reset();
    assert!(
        weak.upgrade().is_none(),
        "T924 origin retains obsolete device/trust metadata"
    );
    let replacement = model.publish(snapshot(1, State::Authenticated)).unwrap();
    assert!(!lifetime.same_as(&replacement.lifetime()));
}
