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
