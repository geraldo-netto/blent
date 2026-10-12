use super::*;

#[test]
fn t1014_watch_coalescing_cannot_revive_a_worker_while_original_retirement_is_held() {
    let mut feed = Feed::new(7, capabilities(), Settings::default()).unwrap();
    feed.observe(event(1, running())).unwrap();
    let (publisher, subscriber) = tokio::sync::watch::channel(feed.snapshot(true, false));
    let started = subscriber.borrow().demand.stamp;
    let mut gate = Gate::default();
    let pool = Pool::new(1).unwrap();
    let (native, completion) = proof(&pool);
    let ticket = gate.reserve(true, 9, completion, &Current(9)).unwrap();
    feed.observe(event(
        2,
        HostState {
            display_on: Some(false),
            ..running()
        },
    ))
    .unwrap();
    publisher.send_replace(feed.snapshot(true, false));
    feed.observe(event(3, running())).unwrap();
    publisher.send_replace(feed.snapshot(true, false));
    let latest = *subscriber.borrow();
    assert!(latest.demand.display);
    assert!(latest.must_retire(ActivityTarget::Display, started));
    assert_eq!(
        gate.plan(false, Some(9), &Current(9)).unwrap(),
        Plan::Stop(ticket)
    );
    assert_eq!(
        gate.plan(true, Some(9), &Current(9)).unwrap(),
        Plan::Waiting(ticket)
    );
    drop(native);
    assert_eq!(gate.plan(true, Some(9), &Current(0)).unwrap(), Plan::Idle);
    assert_eq!(
        gate.plan(true, Some(9), &Current(9)).unwrap(),
        Plan::Start { lifetime: 9 }
    );
}

#[test]
fn t1014_rejected_and_terminal_sources_cannot_change_or_restore_demand() {
    assert!(Feed::new(0, capabilities(), Settings::default()).is_err());
    let mut feed = Feed::new(7, capabilities(), Settings::default()).unwrap();
    feed.observe(event(1, running())).unwrap();
    let before = feed.snapshot(true, false);
    for input in [
        event(0, running()),
        event(1, running()),
        Event {
            epoch: 0,
            ..event(2, running())
        },
        Event {
            epoch: 8,
            ..event(2, running())
        },
    ] {
        assert!(feed.observe(input).is_err());
        assert_eq!(feed.snapshot(true, false), before);
    }
    feed.lost();
    let lost = feed.snapshot(true, false);
    assert!(lost.source_lost);
    assert_eq!(lost.demand.reason, Some(Reason::PowerUnknown));
    assert!(!lost.demand.display && !lost.demand.auxiliary);
    assert!(feed.observe(event(u64::MAX, running())).is_err());
    feed.lost();
    assert_eq!(feed.snapshot(true, false), lost);
    let mut unsupported = Feed::new(7, Capabilities::default(), Settings::default()).unwrap();
    unsupported.lost();
    assert!(unsupported.snapshot(true, false).demand.display);
    assert!(unsupported.snapshot(true, false).demand.auxiliary);
}

#[test]
fn t1014_snapshot_rejects_foreign_future_and_inconsistent_contracts() {
    let mut feed = Feed::new(7, capabilities(), Settings::default()).unwrap();
    feed.observe(event(u64::MAX - 1, running())).unwrap();
    let snapshot = feed.snapshot(true, false);
    assert!(snapshot.validate().is_ok());
    for stamp in [
        Stamp {
            epoch: 0,
            revision: 0,
        },
        Stamp {
            epoch: 8,
            revision: 0,
        },
        Stamp {
            epoch: 7,
            revision: u64::MAX,
        },
    ] {
        assert!(snapshot.must_retire(ActivityTarget::Display, stamp));
        let invalid = ActivitySnapshot {
            display_stop: stamp,
            ..snapshot
        };
        assert!(invalid.validate().is_err());
        assert!(invalid.must_retire(ActivityTarget::Auxiliary, snapshot.demand.stamp));
        assert!(ActivitySnapshot {
            auxiliary_stop: stamp,
            ..snapshot
        }
        .validate()
        .is_err());
    }
    let demand = Demand {
        auxiliary: false,
        ..snapshot.demand
    };
    assert!(ActivitySnapshot { demand, ..snapshot }.validate().is_err());
    let demand = Demand {
        stamp: Stamp {
            epoch: 0,
            revision: 0,
        },
        ..snapshot.demand
    };
    assert!(ActivitySnapshot { demand, ..snapshot }.validate().is_err());
    feed.observe(event(
        u64::MAX,
        HostState {
            power: Some(Power::Unavailable),
            ..running()
        },
    ))
    .unwrap();
    let exhausted = feed.snapshot(true, true);
    assert!(exhausted.validate().is_ok());
    assert!(exhausted.must_retire(ActivityTarget::Auxiliary, snapshot.demand.stamp));
}

fn settings(mask: u8) -> Settings {
    Settings {
        pause_on_lock: mask & 1 != 0,
        pause_on_display_off: mask & 2 != 0,
        pause_without_viewer: mask & 4 != 0,
    }
}
fn state(index: u8) -> HostState {
    HostState {
        power: [
            None,
            Some(Power::Running),
            Some(Power::Suspending),
            Some(Power::Unavailable),
        ][(index % 4) as usize],
        locked: [None, Some(false), Some(true)][(index / 4 % 3) as usize],
        display_on: [None, Some(false), Some(true)][(index / 12) as usize],
    }
}
fn corpus(mask: u8, preferences: u8) {
    let caps = Capabilities::from_mask(mask).unwrap();
    let mut feed = Feed::new(7, caps, settings(preferences)).unwrap();
    for index in 0..36 {
        let before = feed.snapshot(true, false);
        let input = event(index as u64 + 1, state(index));
        if policy::validate(input.state, caps).is_err() {
            assert!(feed.observe(input).is_err());
            assert_eq!(before, feed.snapshot(true, false));
        } else {
            feed.observe(input).unwrap();
        }
        let snapshot = feed.snapshot(false, true);
        assert!(snapshot.validate().is_ok());
        assert!(snapshot.demand.unchanged);
    }
    feed.lost();
    assert!(feed.snapshot(true, false).validate().is_ok());
}
#[test]
fn t1014_bounded_capability_preference_and_observation_properties_preserve_contracts() {
    for mask in 0..8 {
        for preferences in 0..8 {
            corpus(mask, preferences);
        }
    }
}
