//! T1137: expiring a controller budget cannot settle blocked original work.
use super::*;

#[tokio::test(start_paused = true)]
async fn t1137_cleanup_budget_preserves_blocked_original_work_until_actual_join() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("original");
    let lease = File::create(&path).unwrap();
    assert_eq!(unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX) }, 0);
    let gate_path = root.path().join("gate");
    let gate = File::create(&gate_path).unwrap();
    assert_eq!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_EX) }, 0);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let report = tests::new_report();
    let original = report
        .reserve_native(control::NativeResource::Producer)
        .unwrap();
    let native = crate::owned_task::Task::spawn_blocking(move || {
        let _lease = lease;
        let gate = File::open(gate_path)?;
        let _ = entered.send(());
        anyhow::ensure!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_SH) } == 0);
        Ok(())
    })
    .unwrap();
    let completion = native.completion();
    original.bind(completion.clone());
    let original = report
        .reserve_native(control::NativeResource::Producer)
        .unwrap();
    // Deliberately unresponsive controller: it awaits blocked original native work.
    let controller = crate::owned_task::Task::spawn(async move { native.await? }).unwrap();
    original.bind(controller.completion());
    let mut producers = Producers::new();
    producers.push(controller);
    ready.await.unwrap();
    let mut deadline = Box::pin(finish_outputs(&mut producers));
    assert!(deadline.as_mut().now_or_never().is_none());
    // Tokio disables automatic virtual-time advance while blocking work runs.
    tokio::time::advance(std::time::Duration::from_secs(4)).await;
    let outcome = deadline.await;
    let probe = File::open(path).unwrap();
    let retained = !completion.is_complete()
        && unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0;
    drop(gate); // Independent sponsor releases the actual kernel wait before assertions.
    report.wait_native().await;
    assert!(completion.is_complete());
    assert_eq!(
        unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    assert!(outcome.is_err(), "T1137 cleanup budget reported success");
    assert!(
        retained,
        "T1137 budget expired before original native settlement"
    );
    assert!(producers.is_empty());
}
