//! T1043: independent desired-work and retained stop-fence oracles.
use super::*;

fn running(capabilities: Capabilities) -> Publication {
    Publication::Observed {
        capabilities,
        snapshot: ActivitySnapshot {
            demand: Demand {
                stamp: Stamp {
                    epoch: 1,
                    revision: 1,
                },
                display: true,
                auxiliary: true,
                unchanged: false,
                reason: None,
            },
            display_stop: Stamp {
                epoch: 1,
                revision: 0,
            },
            auxiliary_stop: Stamp {
                epoch: 1,
                revision: 0,
            },
            source_lost: false,
        },
    }
}
fn all() -> Capabilities {
    Capabilities {
        power: true,
        lock: true,
        display: true,
    }
}
#[test]
fn t1043_pending_failed_stopped_and_supported_source_loss_deny_work() {
    for target in [ActivityTarget::Display, ActivityTarget::Auxiliary] {
        for publication in [
            Publication::Pending,
            Publication::Failed,
            Publication::Stopped,
        ] {
            assert!(WorkObservation::begin(target, Some(publication), true).is_none());
        }
        assert!(WorkObservation::begin(target, Some(running(all())), false).is_none());
        let Publication::Observed {
            capabilities,
            mut snapshot,
        } = running(all())
        else {
            unreachable!()
        };
        snapshot.source_lost = true;
        assert!(WorkObservation::begin(
            target,
            Some(Publication::Observed {
                capabilities,
                snapshot
            }),
            true
        )
        .is_none());
    }
}
#[test]
fn t1043_explicit_unsupported_compositions_and_target_capabilities_preserve_fallback() {
    for target in [ActivityTarget::Display, ActivityTarget::Auxiliary] {
        let absent = WorkObservation::begin(target, None, false).unwrap();
        assert!(absent.permits(target, None, false));
        let Publication::Observed {
            capabilities,
            mut snapshot,
        } = running(Capabilities::default())
        else {
            unreachable!()
        };
        snapshot.source_lost = true;
        let unsupported = Some(Publication::Observed {
            capabilities,
            snapshot,
        });
        let work = WorkObservation::begin(target, unsupported, false).unwrap();
        assert!(work.permits(target, unsupported, false));
        assert!(!work.permits(target, Some(running(all())), true));
    }
    let lock_only = Some(running(Capabilities {
        lock: true,
        ..Default::default()
    }));
    assert!(WorkObservation::begin(ActivityTarget::Auxiliary, lock_only, false).is_some());
    assert!(WorkObservation::begin(ActivityTarget::Display, lock_only, false).is_none());
}
fn event(revision: u64, power: Power, locked: bool) -> Event {
    Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(power),
            locked: Some(locked),
            display_on: Some(true),
        },
    }
}
#[test]
fn t1043_coalesced_pause_resume_invalidates_only_the_affected_original_work() {
    let mut feed = Feed::new(1, all(), Settings::default()).unwrap();
    feed.observe(event(1, Power::Running, false)).unwrap();
    let first = Some(Publication::Observed {
        capabilities: all(),
        snapshot: feed.snapshot(true, false),
    });
    let display = WorkObservation::begin(ActivityTarget::Display, first, true).unwrap();
    let auxiliary = WorkObservation::begin(ActivityTarget::Auxiliary, first, true).unwrap();
    assert!(display.permits(ActivityTarget::Display, first, true));
    assert!(!display.permits(ActivityTarget::Auxiliary, first, true));
    feed.observe(event(2, Power::Running, true)).unwrap();
    assert!(WorkObservation::begin(
        ActivityTarget::Display,
        Some(Publication::Observed {
            capabilities: all(),
            snapshot: feed.snapshot(true, false)
        }),
        true
    )
    .is_none());
    feed.observe(event(3, Power::Running, false)).unwrap();
    let resumed = Some(Publication::Observed {
        capabilities: all(),
        snapshot: feed.snapshot(true, false),
    });
    assert!(!display.permits(ActivityTarget::Display, resumed, true));
    assert!(auxiliary.permits(ActivityTarget::Auxiliary, resumed, true));
    let fresh = WorkObservation::begin(ActivityTarget::Display, resumed, true).unwrap();
    assert!(fresh.permits(ActivityTarget::Display, resumed, true));
    feed.observe(event(4, Power::Suspending, false)).unwrap();
    let paused = Some(Publication::Observed {
        capabilities: all(),
        snapshot: feed.snapshot(true, false),
    });
    assert!(WorkObservation::begin(ActivityTarget::Display, paused, true).is_none());
    assert!(WorkObservation::begin(ActivityTarget::Auxiliary, paused, true).is_none());
    feed.observe(event(5, Power::Running, false)).unwrap();
    let latest = Some(Publication::Observed {
        capabilities: all(),
        snapshot: feed.snapshot(true, false),
    });
    assert!(!fresh.permits(ActivityTarget::Display, latest, true));
    assert!(!auxiliary.permits(ActivityTarget::Auxiliary, latest, true));
}
#[test]
fn t1043_bounded_invalid_epoch_revision_and_stop_fences_fail_closed() {
    for epoch in 0..=3 {
        for revision in 0..=8 {
            for stopped in 0..=10 {
                let Publication::Observed {
                    capabilities,
                    mut snapshot,
                } = running(all())
                else {
                    unreachable!()
                };
                snapshot.demand.stamp = Stamp { epoch, revision };
                snapshot.display_stop = Stamp {
                    epoch,
                    revision: stopped,
                };
                snapshot.auxiliary_stop = snapshot.display_stop;
                let valid = epoch > 0 && stopped <= revision;
                let actual = WorkObservation::begin(
                    ActivityTarget::Display,
                    Some(Publication::Observed {
                        capabilities,
                        snapshot,
                    }),
                    true,
                );
                assert_eq!(
                    actual.is_some(),
                    valid,
                    "T1043 epoch={epoch} revision={revision} stop={stopped}"
                );
            }
        }
    }
}
#[test]
fn t1043_foreign_future_inconsistent_or_closed_publications_cannot_retain_old_work() {
    let initial = Some(running(all()));
    let original = WorkObservation::begin(ActivityTarget::Display, initial, true).unwrap();
    for field in 0..7 {
        let Publication::Observed {
            capabilities,
            mut snapshot,
        } = running(all())
        else {
            unreachable!()
        };
        match field {
            0 => snapshot.demand.stamp.epoch = 2,
            1 => snapshot.demand.stamp.revision = 0,
            2 => snapshot.display_stop.epoch = 2,
            3 => snapshot.auxiliary_stop.revision = 2,
            4 => snapshot.demand.reason = Some(Reason::Locked),
            5 => snapshot.demand.auxiliary = false,
            _ => snapshot.source_lost = true,
        }
        assert!(!original.permits(
            ActivityTarget::Display,
            Some(Publication::Observed {
                capabilities,
                snapshot
            }),
            true
        ));
    }
    assert!(!original.permits(ActivityTarget::Display, initial, false));
    assert!(!original.permits(ActivityTarget::Display, Some(Publication::Stopped), true));
    assert!(!original.permits(
        ActivityTarget::Display,
        Some(running(Capabilities::default())),
        true
    ));
}

#[test]
fn t1043_capability_and_source_state_matrix_preserves_explicit_target_fallback() {
    const SUPPORTED: [[bool; 2]; 8] = [
        [false, false],
        [true, true],
        [true, false],
        [true, true],
        [true, false],
        [true, true],
        [true, false],
        [true, true],
    ];
    for (mask, expected_targets) in SUPPORTED.iter().enumerate() {
        for (target, supported) in [ActivityTarget::Display, ActivityTarget::Auxiliary]
            .into_iter()
            .zip(expected_targets)
        {
            for open in [false, true] {
                for lost in [false, true] {
                    let Publication::Observed {
                        capabilities,
                        mut snapshot,
                    } = running(Capabilities::from_mask(mask as u8).unwrap())
                    else {
                        unreachable!()
                    };
                    snapshot.source_lost = lost;
                    let observed = Some(Publication::Observed {
                        capabilities,
                        snapshot,
                    });
                    assert_eq!(
                        WorkObservation::begin(target, observed, open).is_some(),
                        !supported || (open && !lost)
                    );
                }
            }
        }
    }
}

#[test]
fn t1043_local_viewer_demand_is_independent_of_backend_support() {
    let feed = Feed::new(1, Capabilities::default(), Settings::default()).unwrap();
    let no_viewer = Some(Publication::Observed {
        capabilities: Capabilities::default(),
        snapshot: feed.snapshot(false, false),
    });
    assert!(WorkObservation::begin(ActivityTarget::Display, no_viewer, false).is_none());
    assert!(WorkObservation::begin(ActivityTarget::Auxiliary, no_viewer, false).is_some());
}
