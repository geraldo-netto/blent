use super::*;
use std::{
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};

fn wait_closed(owners: &Owners) -> bool {
    let start = Instant::now();
    while !*owners.closed.lock().unwrap() {
        if start.elapsed() >= Duration::from_secs(3) {
            return false;
        }
        thread::sleep(Duration::from_millis(1));
    }
    true
}
fn native(admission: Admission) -> mpsc::Sender<()> {
    let (release, blocked) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = blocked.recv();
    });
    admission.retain(blent_config::retirement_work::Thread::new(worker));
    release
}

#[test]
fn t1410_process_owner_rejects_capacity_and_post_cutoff_admission() {
    for limit in [0, 4097, usize::MAX] {
        assert!(Owners::new(limit).is_err());
    }
    let owners = Owners::new(1).unwrap();
    let admitted = owners.reserve().unwrap();
    assert!(owners.reserve().is_err());
    drop(admitted);
    drop(owners.reserve().unwrap());
    owners.shutdown();
    assert!(owners.reserve().is_err());
}

#[test]
fn t1410_shutdown_closes_admission_and_waits_for_original_native_exit() {
    let owners = Arc::new(Owners::new(1).unwrap());
    let release = native(owners.reserve().unwrap());
    let original = owners.clone();
    let shutdown = thread::spawn(move || original.shutdown());
    let closed = wait_closed(&owners);
    let retained = !shutdown.is_finished();
    let rejected = owners.reserve().is_err();
    drop(release);
    shutdown.join().unwrap();
    assert!(
        closed && retained && rejected,
        "T1410 process cutoff did not retain original worker"
    );
    assert_eq!(owners.pool.active(), 0);
}

fn exit_child() {
    drop(ProcessExit); // Unused input must not allocate a new retirement pool.
    assert!(OWNERS.get().is_none());
    let release = native(reserve().unwrap());
    let shutdown = thread::spawn(|| drop(ProcessExit));
    let closed = wait_closed(Owners::shared().unwrap());
    let retained = !shutdown.is_finished();
    let rejected = reserve().is_err();
    drop(release);
    shutdown.join().unwrap();
    assert!(
        closed && retained && rejected,
        "T1410 entry guard bypassed native retirement"
    );
}

#[test]
fn t1410_entry_guard_waits_for_native_exit_in_isolated_process() {
    const CHILD: &str = "BLENT_T1410_INPUT_EXIT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        exit_child();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "input::direct::ownership::tests::t1410_entry_guard_waits_for_native_exit_in_isolated_process", "--nocapture"])
        .env(CHILD, "1").output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
