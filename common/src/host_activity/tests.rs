use super::*;
use crate::retirement::{Admission, Completion, Observe, Pool};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
mod feed;

fn capabilities() -> Capabilities {
    Capabilities {
        power: true,
        lock: true,
        display: true,
    }
}
fn running() -> HostState {
    HostState {
        power: Some(Power::Running),
        locked: Some(false),
        display_on: Some(true),
    }
}
fn event(revision: u64, state: HostState) -> Event {
    Event {
        epoch: 7,
        revision,
        state,
    }
}
#[test]
fn t1014_coalesced_pause_resume_still_retires_only_the_original_affected_worker() {
    let mut feed = Feed::new(7, capabilities(), Settings::default()).unwrap();
    feed.observe(event(1, running())).unwrap();
    let started = feed.snapshot(true, false).demand.stamp;
    feed.observe(event(
        2,
        HostState {
            locked: Some(true),
            ..running()
        },
    ))
    .unwrap();
    feed.observe(event(3, running())).unwrap();
    let latest = feed.snapshot(true, false);
    assert!(latest.demand.display && latest.demand.auxiliary);
    assert!(latest.must_retire(ActivityTarget::Display, started));
    assert!(!latest.must_retire(ActivityTarget::Auxiliary, started));
    assert!(!latest.must_retire(ActivityTarget::Display, latest.demand.stamp));
    feed.observe(event(
        4,
        HostState {
            power: Some(Power::Suspending),
            ..running()
        },
    ))
    .unwrap();
    feed.observe(event(5, running())).unwrap();
    let latest = feed.snapshot(true, false);
    assert!(latest.must_retire(ActivityTarget::Display, started));
    assert!(latest.must_retire(ActivityTarget::Auxiliary, started));
}
struct Current(u64);
impl Authority for Current {
    fn current(&self, lifetime: u64) -> bool {
        self.0 == lifetime
    }
}
fn proof(pool: &Pool) -> (Admission, Completion) {
    let owner = pool.reserve().unwrap();
    let completion = owner.completion();
    (owner, completion)
}

#[test]
fn t913_lock_display_off_and_viewer_loss_pause_only_display() {
    let mut policy = Policy::new(7, capabilities(), Settings::default()).unwrap();
    assert_eq!(
        policy.demand(true, false).reason,
        Some(Reason::PowerUnknown)
    );
    policy.observe(event(1, running())).unwrap();
    let active = policy.demand(true, false);
    assert!(active.display && active.auxiliary);
    assert!(!active.unchanged);
    assert_eq!(
        active.stamp,
        Stamp {
            epoch: 7,
            revision: 1
        }
    );
    for (revision, state, reason) in [
        (
            2,
            HostState {
                locked: Some(true),
                ..running()
            },
            Reason::Locked,
        ),
        (
            3,
            HostState {
                display_on: Some(false),
                ..running()
            },
            Reason::DisplayOff,
        ),
        (
            4,
            HostState {
                locked: None,
                ..running()
            },
            Reason::LockUnknown,
        ),
        (
            5,
            HostState {
                display_on: None,
                ..running()
            },
            Reason::DisplayUnknown,
        ),
    ] {
        policy.observe(event(revision, state)).unwrap();
        let paused = policy.demand(true, true);
        assert!(!paused.display);
        assert!(paused.auxiliary);
        assert!(paused.unchanged);
        assert_eq!(paused.reason, Some(reason));
    }
    policy.observe(event(6, running())).unwrap();
    assert_eq!(policy.demand(false, false).reason, Some(Reason::NoViewer));
    assert!(policy.demand(false, false).auxiliary);
    assert!(
        policy.demand(true, true).display,
        "T913 unchanged content belongs to frame cadence, not host absence"
    );
}

#[test]
fn t913_suspend_unavailability_and_unknown_supported_power_interrupt_every_resource() {
    let mut policy = Policy::new(
        7,
        capabilities(),
        Settings {
            pause_on_lock: false,
            pause_on_display_off: false,
            pause_without_viewer: false,
        },
    )
    .unwrap();
    for (revision, power, reason) in [
        (1, Some(Power::Suspending), Reason::Suspending),
        (2, Some(Power::Unavailable), Reason::Unavailable),
        (3, None, Reason::PowerUnknown),
    ] {
        policy
            .observe(event(revision, HostState { power, ..running() }))
            .unwrap();
        let paused = policy.demand(false, false);
        assert!(!paused.display && !paused.auxiliary);
        assert_eq!(paused.reason, Some(reason));
    }
    policy.observe(event(4, running())).unwrap();
    assert!(policy.demand(false, false).display && policy.demand(false, false).auxiliary);
}

#[test]
fn t913_invalid_stale_foreign_and_exhausted_event_corpus_is_atomic() {
    assert!(Policy::new(0, capabilities(), Settings::default()).is_err());
    let mut policy = Policy::new(7, capabilities(), Settings::default()).unwrap();
    policy.observe(event(1, running())).unwrap();
    let original = policy.demand(true, false);
    for epoch in [0, 1, 6, 8, u64::MAX] {
        assert!(policy
            .observe(Event {
                epoch,
                revision: 2,
                state: HostState::default()
            })
            .is_err());
        assert_eq!(original, policy.demand(true, false));
    }
    for revision in [0, 1] {
        assert!(policy
            .observe(event(revision, HostState::default()))
            .is_err());
        assert_eq!(original, policy.demand(true, false));
    }
    policy.observe(event(u64::MAX, running())).unwrap();
    assert!(policy.observe(event(0, HostState::default())).is_err());
    assert!(policy
        .observe(event(u64::MAX, HostState::default()))
        .is_err());
    assert_eq!(u64::MAX, policy.demand(true, false).stamp.revision);
    assert!(Policy::new(u64::MAX, Capabilities::default(), Settings::default()).is_ok());
}

#[test]
fn t913_capability_and_setting_boundaries_preserve_explicit_unsupported_fallback() {
    for mask in 0u8..=255 {
        let decoded = Capabilities::from_mask(mask);
        assert_eq!(mask <= 7, decoded.is_ok());
        if let Ok(capabilities) = decoded {
            setting_cases(capabilities);
        }
    }
    let policy = Policy::new(7, Capabilities::default(), Settings::default()).unwrap();
    assert_eq!(policy.capabilities(), Capabilities::default());
    assert!(policy.demand(true, false).display && policy.demand(true, false).auxiliary);
    for state in [
        running(),
        HostState {
            power: Some(Power::Unavailable),
            ..Default::default()
        },
        HostState {
            locked: Some(true),
            ..Default::default()
        },
        HostState {
            display_on: Some(false),
            ..Default::default()
        },
    ] {
        let mut unsupported = Policy::new(7, Capabilities::default(), Settings::default()).unwrap();
        assert!(unsupported.observe(event(1, state)).is_err());
        assert_eq!(0, unsupported.demand(true, false).stamp.revision);
    }
}
fn setting_cases(capabilities: Capabilities) {
    for flags in 0..8 {
        let settings = Settings {
            pause_on_lock: flags & 1 != 0,
            pause_on_display_off: flags & 2 != 0,
            pause_without_viewer: flags & 4 != 0,
        };
        let mut policy = Policy::new(7, capabilities, settings).unwrap();
        let state = HostState {
            power: capabilities.power.then_some(Power::Running),
            locked: capabilities.lock.then_some(true),
            display_on: capabilities.display.then_some(false),
        };
        policy.observe(event(1, state)).unwrap();
        for viewer in [false, true] {
            let display = !(settings.pause_on_lock && capabilities.lock
                || settings.pause_on_display_off && capabilities.display
                || settings.pause_without_viewer && !viewer);
            assert_eq!(display, policy.demand(viewer, false).display);
            assert!(policy.demand(viewer, true).auxiliary);
        }
    }
}

#[test]
fn t913_settings_round_trip_and_invalid_types_do_not_reconfigure_unrelated_media() {
    let baseline = crate::FileConfig::default();
    assert_eq!(baseline.host_activity, Settings::default());
    for flags in 0..8 {
        let settings = Settings {
            pause_on_lock: flags & 1 != 0,
            pause_on_display_off: flags & 2 != 0,
            pause_without_viewer: flags & 4 != 0,
        };
        let selected = crate::FileConfig {
            host_activity: settings,
            ..baseline.clone()
        };
        let saved = toml::to_string(&selected).unwrap();
        let restored: crate::FileConfig = toml::from_str(&saved).unwrap();
        assert_eq!(selected, restored);
        assert_eq!(baseline.camera, restored.camera);
        assert_eq!(baseline.audio, restored.audio);
    }
    for field in [
        "pause_on_lock",
        "pause_on_display_off",
        "pause_without_viewer",
    ] {
        for value in ["0", "-1", "9223372036854775807", "'false'", "[]", "{}"] {
            assert!(toml::from_str::<crate::FileConfig>(&format!(
                "[host_activity]\n{field}={value}\n"
            ))
            .is_err());
        }
    }
    let partial: crate::FileConfig =
        toml::from_str("[host_activity]\npause_on_lock=false\n").unwrap();
    assert!(!partial.host_activity.pause_on_lock);
    assert!(partial.host_activity.pause_on_display_off);
}

#[test]
fn t913_concurrent_settings_edits_merge_by_leaf_without_overwriting_media() {
    let baseline = crate::FileConfig::default();
    let mut edited = baseline.clone();
    edited.host_activity.pause_on_lock = false;
    let mut latest = baseline.clone();
    latest.host_activity.pause_on_display_off = false;
    latest.host_activity.pause_without_viewer = false;
    latest.camera.options.background = true;
    let merged = edited.merge_edits(&baseline, latest.clone()).unwrap();
    assert!(!merged.host_activity.pause_on_lock);
    assert!(!merged.host_activity.pause_on_display_off);
    assert!(!merged.host_activity.pause_without_viewer);
    assert_eq!(merged.camera, latest.camera);
    assert_eq!(merged.audio, latest.audio);
    assert!(merged.requires_restart_from(&baseline));
}

#[test]
fn t913_gate_retains_cancelled_original_until_actual_completion_and_rechecks_authority() {
    let pool = Pool::new(1).unwrap();
    let authority = Current(7);
    let mut gate = Gate::default();
    assert_eq!(gate.plan(false, None, &authority).unwrap(), Plan::Idle);
    assert_eq!(gate.plan(true, None, &authority).unwrap(), Plan::Idle);
    assert_eq!(gate.plan(true, Some(8), &authority).unwrap(), Plan::Idle);
    assert!(gate.plan(true, Some(0), &authority).is_err());
    let (owner, completed) = proof(&pool);
    assert!(gate
        .reserve(false, 7, completed.clone(), &authority)
        .is_err());
    assert!(gate
        .reserve(true, 8, completed.clone(), &authority)
        .is_err());
    let ticket = gate
        .reserve(true, 7, completed.clone(), &authority)
        .unwrap();
    assert_eq!(
        gate.plan(true, Some(7), &authority).unwrap(),
        Plan::Keep(ticket)
    );
    assert!(gate
        .reserve(true, 7, completed.clone(), &authority)
        .is_err());
    assert_eq!(
        gate.plan(false, Some(7), &authority).unwrap(),
        Plan::Stop(ticket)
    );
    assert_eq!(
        gate.plan(true, Some(7), &authority).unwrap(),
        Plan::Waiting(ticket)
    );
    assert_eq!(
        gate.plan(false, None, &authority).unwrap(),
        Plan::Waiting(ticket)
    );
    assert!(!completed.is_complete());
    drop(owner);
    assert_eq!(
        gate.plan(true, Some(7), &authority).unwrap(),
        Plan::Start { lifetime: 7 }
    );
    assert!(gate.reserve(true, 7, completed, &authority).is_err());
    let (next_owner, next_proof) = proof(&pool);
    let next = gate.reserve(true, 7, next_proof, &authority).unwrap();
    assert_ne!(ticket, next);
    assert_eq!(
        gate.plan(true, Some(7), &Current(8)).unwrap(),
        Plan::Stop(next)
    );
    drop(next_owner);
    assert_eq!(gate.plan(true, Some(7), &Current(8)).unwrap(), Plan::Idle);
}

#[test]
fn t913_gate_counter_exhaustion_cannot_revive_an_old_request() {
    let pool = Pool::new(1).unwrap();
    let (owner, completion) = proof(&pool);
    let mut gate = Gate::default();
    gate.next = u64::MAX - 1;
    let last = gate.reserve(true, 7, completion, &Current(7)).unwrap();
    assert_eq!(last.request, u64::MAX);
    assert_eq!(
        gate.plan(true, Some(8), &Current(8)).unwrap(),
        Plan::Stop(last)
    );
    assert_eq!(
        gate.plan(true, Some(8), &Current(8)).unwrap(),
        Plan::Waiting(last)
    );
    drop(owner);
    assert!(gate.plan(true, Some(8), &Current(8)).is_err());
    let (held, pending) = proof(&pool);
    assert!(gate.reserve(true, 8, pending, &Current(8)).is_err());
    drop(held);
    assert_eq!(gate.plan(false, None, &Current(8)).unwrap(), Plan::Idle);
}

struct Held(Arc<AtomicBool>);
impl Observe for Held {
    fn poll_complete(&mut self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
#[tokio::test]
async fn t913_stuck_native_retirement_cannot_block_independent_resource_gates() {
    let pool = Pool::new(2).unwrap();
    let (owner, completion) = proof(&pool);
    let released = Arc::new(AtomicBool::new(false));
    owner.retain(Held(released.clone()));
    let mut stuck = Gate::default();
    let mut sibling = Gate::default();
    let ticket = stuck
        .reserve(true, 7, completion.clone(), &Current(7))
        .unwrap();
    assert_eq!(
        stuck.plan(false, None, &Current(7)).unwrap(),
        Plan::Stop(ticket)
    );
    for _ in 0..32 {
        assert_eq!(
            stuck.plan(true, Some(7), &Current(7)).unwrap(),
            Plan::Waiting(ticket)
        );
        let (owner, proof) = proof(&pool);
        let live = sibling.reserve(true, 8, proof, &Current(8)).unwrap();
        assert_eq!(
            sibling.plan(true, Some(8), &Current(8)).unwrap(),
            Plan::Keep(live)
        );
        drop(owner);
        assert_eq!(sibling.plan(false, None, &Current(8)).unwrap(), Plan::Idle);
    }
    assert!(!completion.is_complete());
    released.store(true, Ordering::SeqCst);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !completion.is_complete() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        stuck.plan(true, Some(7), &Current(7)).unwrap(),
        Plan::Start { lifetime: 7 }
    );
}

#[test]
fn t1008_bounded_observations_preserve_transitions_and_fail_closed_on_overflow() {
    let mut observations = Observations::new(7, capabilities(), 2).unwrap();
    assert_eq!(observations.publish(running()).unwrap(), true);
    assert_eq!(observations.publish(running()).unwrap(), false);
    let suspended = HostState {
        power: Some(Power::Suspending),
        ..running()
    };
    observations.publish(suspended).unwrap();
    assert!(observations.publish(running()).is_err());
    let event = observations.poll().unwrap().unwrap();
    assert_eq!(event.epoch, 7);
    assert_eq!(event.revision, 3);
    assert_eq!(event.state, HostState::default());
    assert!(observations.poll().is_err());
    assert!(observations.publish(running()).is_err());
    observations.stop();
    observations.stop();
    assert!(observations.poll().unwrap().is_none());
}
#[test]
fn t1008_observation_boundaries_reject_without_reviving_epochs() {
    for capacity in [0, 257, usize::MAX] {
        assert!(Observations::new(1, capabilities(), capacity).is_err());
    }
    assert!(Observations::new(0, capabilities(), 32).is_err());
    for capacity in [1, 32, 256] {
        let mut observations = Observations::new(u64::MAX, capabilities(), capacity).unwrap();
        observations.publish(running()).unwrap();
        let first = observations.poll().unwrap().unwrap();
        assert_eq!(first.revision, 1);
        assert!(observations.poll().unwrap().is_none());
        observations.revision = u64::MAX;
        assert!(observations.publish(HostState::default()).is_err());
        assert!(observations.poll().is_err());
    }
    let mut unsupported = Observations::new(1, Capabilities::default(), 32).unwrap();
    assert!(unsupported.publish(running()).is_err());
    assert_eq!(unsupported.current(), HostState::default());
}

#[test]
fn t1008_bounded_capability_property_rejects_unsupported_native_fields() {
    for mask in 0..8 {
        let caps = Capabilities::from_mask(mask).unwrap();
        for inputs in 0..8 {
            let state = HostState {
                power: (inputs & 1 != 0).then_some(Power::Running),
                locked: (inputs & 2 != 0).then_some(false),
                display_on: (inputs & 4 != 0).then_some(true),
            };
            let mut observations = Observations::new(1, caps, 1).unwrap();
            assert_eq!(observations.publish(state).is_ok(), inputs & !mask == 0);
            if inputs & !mask != 0 {
                assert!(observations.poll().unwrap().is_none());
            }
        }
    }
}

#[test]
fn t1008_source_settings_contract_validates_independently_of_native_adapters() {
    use std::time::Duration;
    assert!(SourceSettings::default().validate().is_ok());
    for deadline in [
        Duration::from_nanos(1),
        Duration::from_secs(2),
        Duration::from_secs(60),
    ] {
        for queue in [1, 32, 256] {
            assert!(SourceSettings { deadline, queue }.validate().is_ok());
        }
    }
    for deadline in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        assert!(SourceSettings {
            deadline,
            ..SourceSettings::default()
        }
        .validate()
        .is_err());
    }
    for queue in [0, 257, usize::MAX] {
        assert!(SourceSettings {
            queue,
            ..SourceSettings::default()
        }
        .validate()
        .is_err());
    }
}

#[test]
fn t1013_display_observation_contract_rejects_invalid_and_oversized_frames() {
    for supported in [false, true] {
        for on in [None, Some(false), Some(true)] {
            let sample = DisplayObservation { supported, on };
            assert_eq!(sample.encode().is_ok(), supported || on.is_none());
            if let Ok(bytes) = sample.encode() {
                assert_eq!(DisplayObservation::decode(&bytes).unwrap(), sample);
            }
        }
    }
    for bytes in [
        b"".as_slice(),
        b"null",
        b"{",
        b"{\"supported\":false,\"on\":true}",
        b"{\"supported\":true,\"on\":1}",
        b"{\"supported\":true,\"on\":null,\"extra\":1}",
        &[0xff],
    ] {
        assert!(DisplayObservation::decode(bytes).is_err());
    }
    assert!(DisplayObservation::decode(&vec![b' '; 257]).is_err());
}

#[test]
fn t1018_independent_lifetime_contract_requires_every_original_certificate_and_last_clone() {
    for hold_clone in [false, true] {
        let owners = Pool::new(1).unwrap();
        let native = Pool::new(2).unwrap();
        let first = native.reserve().unwrap();
        let second = native.reserve().unwrap();
        let lifetime = SourceLifetime::new(owners.reserve().unwrap());
        lifetime.retain(first.completion());
        lifetime.retain(second.completion());
        let completion = lifetime.completion();
        let mut clone = hold_clone.then(|| lifetime.clone());
        drop(lifetime);
        drop(first);
        assert!(!completion.is_complete());
        assert!(owners.reserve().is_err());
        drop(second);
        if hold_clone {
            assert!(!completion.is_complete());
            assert!(owners.reserve().is_err());
            drop(clone.take());
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !completion.is_complete() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(owners.reserve().is_ok());
    }
}
