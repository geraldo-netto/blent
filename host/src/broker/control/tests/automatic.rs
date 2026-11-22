use super::*;
use blent_config::connection::control::Availability;

fn fixture() -> Rig {
    let mut rig = Rig::new(vec![peer(1)]);
    rig.control.replace_devices(vec![device(1)]).unwrap();
    rig
}
fn start(rig: &mut Rig, now: i64) -> u64 {
    match rig.control.automatic(now).unwrap() {
        Action::Start { connection, target } => {
            assert_eq!(target, 1);
            connection
        }
        _ => panic!("T1391: automatic connection missing"),
    }
}
fn idle(rig: &mut Rig, now: i64) {
    assert!(matches!(rig.control.automatic(now).unwrap(), Action::None));
}

#[test]
fn t1391_startup_and_replug_require_fresh_proof_and_exact_retirement() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    let hold = rig.pool.reserve().unwrap();
    rig.control.attach(first, hold.completion()).unwrap();
    assert!(matches!(
        rig.control.identified(first, peer(1), 1).unwrap(),
        Action::Authenticate { .. }
    ));
    assert!(
        rig.control.lease(first).is_none(),
        "T1391: cached pin is not TLS proof"
    );
    rig.control.replace_devices(vec![]).unwrap();
    rig.control.failed(first).unwrap();
    rig.control.native_finished(first).unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    idle(&mut rig, 3_000);
    assert!(rig.control.collect_retired().unwrap().is_empty());
    drop(hold);
    assert_eq!(rig.control.collect_retired().unwrap(), [first]);
    let next = start(&mut rig, 3_000);
    assert_ne!(first, next);
    let hold = rig.pool.reserve().unwrap();
    rig.control.attach(next, hold.completion()).unwrap();
    assert!(matches!(
        rig.control.identified(next, peer(2), 3_001).unwrap(),
        Action::None
    ));
    assert_eq!(rig.control.links[&next].state, State::Pending);
    assert!(rig.control.lease(next).is_none());
}

#[test]
fn t1391_failure_retries_at_boundary_without_duplicate_owners() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    idle(&mut rig, 3_000);
    rig.control.start_failed(first).unwrap();
    idle(&mut rig, 3_000);
    rig.control.collect_retired().unwrap();
    for now in [0, 1, 1_999] {
        idle(&mut rig, now);
    }
    assert_ne!(start(&mut rig, 2_000), first);
}

#[test]
fn t1391_explicit_cutoff_survives_unknown_inventory_until_fresh_attach() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    rig.disconnect(first, None);
    rig.control.start_failed(first).unwrap();
    rig.control.collect_retired().unwrap();
    rig.control.inventory_unavailable().unwrap();
    idle(&mut rig, 3_000);
    rig.control.replace_devices(vec![device(1)]).unwrap();
    idle(&mut rig, 3_000);
    rig.control.replace_devices(vec![]).unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    start(&mut rig, 3_000);
}

#[test]
fn t1391_denial_and_revoke_preserve_explicit_cutoff() {
    for revoke in [false, true] {
        let mut rig = fixture();
        let (id, ticket, hold) = rig.pending(1, peer(2));
        if revoke {
            rig.revoke(peer(2));
            let job = rig.control.next_authority().unwrap().unwrap();
            rig.control
                .complete_authority(job.run(&mut rig.trust), 6)
                .unwrap();
        } else {
            rig.control
                .command(
                    rig.client,
                    Command::Decide {
                        connection: id,
                        ticket,
                        allow: false,
                        remember: false,
                    },
                    2,
                )
                .unwrap();
        }
        rig.control.native_finished(id).unwrap();
        drop(hold);
        rig.control.collect_retired().unwrap();
        idle(&mut rig, 3_000);
        // A manual request deliberately overrides the current cutoff.
        rig.start(1);
    }
}

#[test]
fn t1391_automatic_admission_respects_trust_availability_and_bounds() {
    let mut rig = Rig::new(vec![]);
    idle(&mut rig, 0);
    rig.control.saved.push(peer(1));
    rig.control.healthy = false;
    idle(&mut rig, 0);
    rig.control.healthy = true;
    let mut reserved = device(1);
    reserved.availability = Availability::Debugging;
    rig.control.replace_devices(vec![reserved]).unwrap();
    idle(&mut rig, 0);
    rig.control.replace_devices(vec![device(1)]).unwrap();
    for now in [-1, i64::MIN, i64::MAX] {
        assert!(rig.control.automatic(now).is_err());
    }
    rig.control.limits.links = 1;
    start(&mut rig, 0);
    idle(&mut rig, 3_000);
}

#[test]
fn t1594_unrelated_usb_devices_are_never_automatic_candidates() {
    let mut rig = fixture();
    let mut keyboard = device(2);
    keyboard.vendor = 0x046d;
    keyboard.product = 0xc31c;
    keyboard.accessory = false;
    let mut phone = device(3);
    phone.vendor = 0x1234;
    phone.product = 7;
    phone.accessory = false;
    rig.control
        .replace_devices(vec![keyboard, phone, device(1)])
        .unwrap();
    start(&mut rig, 0);
    idle(&mut rig, 60_000);
}

#[test]
fn t1594_repeated_start_failures_back_off_to_a_bounded_interval() {
    let mut rig = fixture();
    let mut now = 0;
    let mut current = start(&mut rig, now);
    for delay in [2_000, 4_000, 8_000, 16_000, 32_000, 60_000, 60_000] {
        rig.control.start_failed(current).unwrap();
        rig.control.collect_retired().unwrap();
        idle(&mut rig, now + delay - 1);
        now += delay;
        current = start(&mut rig, now);
    }
}

#[test]
fn t1594_discovery_failure_suppresses_automatic_retry_until_confirmed_replug() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    let hold = rig.pool.reserve().unwrap();
    rig.control.attach(first, hold.completion()).unwrap();
    rig.control.failed(first).unwrap();
    rig.control.native_finished(first).unwrap();
    idle(&mut rig, 1_000_000);
    assert!(rig.control.collect_retired().unwrap().is_empty());
    drop(hold);
    rig.control.collect_retired().unwrap();
    idle(&mut rig, 1_000_000);
    rig.control.inventory_unavailable().unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    idle(&mut rig, 1_000_000);
    rig.control.replace_devices(vec![]).unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    start(&mut rig, 1_000_000);
}

#[test]
fn t1594_detach_during_original_discovery_cleanup_clears_retry_history() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    let hold = rig.pool.reserve().unwrap();
    rig.control.attach(first, hold.completion()).unwrap();
    rig.control.failed(first).unwrap();
    rig.control.replace_devices(vec![]).unwrap();
    rig.control.replace_devices(vec![device(1)]).unwrap();
    rig.control.native_finished(first).unwrap();
    idle(&mut rig, 60_000);
    drop(hold);
    rig.control.collect_retired().unwrap();
    start(&mut rig, 60_000);
}

#[test]
fn t1594_authentication_resets_backoff_before_next_connection() {
    let mut rig = fixture();
    let first = start(&mut rig, 0);
    rig.control.start_failed(first).unwrap();
    rig.control.collect_retired().unwrap();
    let (id, ticket, hold) = rig.pending(1, peer(1));
    let job = rig.job(id, &ticket);
    assert!(rig
        .control
        .complete_authority(job.run(&mut rig.trust), 4)
        .unwrap()
        .is_some());
    rig.control.failed(id).unwrap();
    rig.control.native_finished(id).unwrap();
    drop(hold);
    rig.control.collect_retired().unwrap();
    let next = start(&mut rig, 2_000);
    rig.control.start_failed(next).unwrap();
    rig.control.collect_retired().unwrap();
    idle(&mut rig, 3_999);
    start(&mut rig, 4_000);
}

#[test]
fn t1777_retry_deadline_matches_admission_and_suppression() {
    let mut rig = fixture();
    assert_eq!(rig.control.automatic_deadline(), Some(0));
    let first = start(&mut rig, 10);
    assert_eq!(rig.control.automatic_deadline(), None);
    rig.control.start_failed(first).unwrap();
    rig.control.collect_retired().unwrap();
    assert_eq!(rig.control.automatic_deadline(), Some(2010));
    idle(&mut rig, 2009);
    rig.control.automatic.get_mut(&1).unwrap().suppress();
    assert_eq!(rig.control.automatic_deadline(), None);
    rig.control.inventory_unavailable().unwrap();
    assert_eq!(rig.control.automatic_deadline(), None);
}

#[test]
fn t1777_deadline_excludes_unavailable_and_non_accessory_devices() {
    let mut rig = fixture();
    for available in [false, true] {
        for accessory in [false, true] {
            let mut candidate = device(1);
            candidate.accessory = accessory;
            candidate.availability = if available {
                Availability::Available
            } else {
                Availability::Debugging
            };
            rig.control.replace_devices(vec![candidate]).unwrap();
            assert_eq!(
                rig.control.automatic_deadline().is_some(),
                available && accessory
            );
        }
    }
    rig.control.healthy = false;
    assert_eq!(rig.control.automatic_deadline(), None);
    rig.control.healthy = true;
    rig.control.saved.clear();
    assert_eq!(rig.control.automatic_deadline(), None);
}

#[test]
fn t1777_expiry_stops_at_deadline_and_retirement_keeps_original_fence() {
    let mut rig = fixture();
    let first = start(&mut rig, 10);
    let deadline = rig.control.expiry_deadline().unwrap();
    let hold = rig.pool.reserve().unwrap();
    rig.control.attach(first, hold.completion()).unwrap();
    assert!(!rig.control.retirement_pending());
    assert!(rig.control.tick(deadline - 1).unwrap().is_empty());
    assert_eq!(rig.control.tick(deadline).unwrap(), [first]);
    assert_eq!(rig.control.expiry_deadline(), None);
    assert!(rig.control.retirement_pending());
    rig.control.native_finished(first).unwrap();
    assert!(rig.control.collect_retired().unwrap().is_empty());
    drop(hold);
    assert_eq!(rig.control.collect_retired().unwrap(), [first]);
    assert!(!rig.control.retirement_pending());
}
