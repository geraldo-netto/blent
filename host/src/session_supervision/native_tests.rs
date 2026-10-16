//! T1026: task results cannot certify original dependent native cleanup.
use super::tests::admission;
use super::*;

async fn completed(original: &blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

async fn parent_result(mode: u8) {
    let sessions = Pool::new(2).unwrap();
    let native = Pool::new(1).unwrap();
    let original = native.reserve().unwrap();
    let proof = original.completion();
    let mut workers = Workers::empty(admission(&sessions));
    let report = workers.capture_lifetime();
    let (started, entered) = tokio::sync::oneshot::channel();
    workers.add(
        "Capture",
        tokio::spawn(async move {
            report.retain(proof);
            started.send(()).unwrap();
            match mode {
                0 => Ok(()),
                1 => anyhow::bail!("T1026 parent failed with original native cleanup held"),
                2 => panic!("T1026 parent panicked with original native cleanup held"),
                _ => std::future::pending().await,
            }
        }),
    );
    entered.await.unwrap();
    let completion = workers.completion();
    workers.retire().await;
    drop(workers);
    let retained = !completion.is_complete();
    let mut sibling = Workers::new(admission(&sessions), tokio::spawn(async { Ok(()) }), vec![]);
    let capacity_held = sessions.reserve().is_err();
    sibling.retire().await;
    drop(sibling);
    drop(original);
    completed(&completion).await;
    assert!(
        retained,
        "T1026 parent result released original native retirement"
    );
    assert!(
        capacity_held,
        "T1026 incomplete native owner released session capacity"
    );
    assert_eq!(sessions.active(), 0);
}

#[tokio::test]
async fn t1026_success_error_panic_and_cancellation_retain_original_native() {
    for mode in 0..4 {
        parent_result(mode).await;
    }
}

#[tokio::test]
async fn t1026_dropped_owner_retains_late_original_creator_and_native() {
    let sessions = Pool::new(1).unwrap();
    let native = Pool::new(1).unwrap();
    let mut workers = Workers::empty(admission(&sessions));
    let creator = workers.capture_lifetime();
    let report = creator.clone();
    let (started, entered) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let (born, received) = tokio::sync::oneshot::channel();
    let task = crate::owned_task::Task::spawn_blocking(move || {
        started.send(()).unwrap();
        held.recv_timeout(Duration::from_secs(5)).unwrap();
        let original = native.reserve().unwrap();
        creator.retain(original.completion());
        born.send(original).unwrap();
        Ok(())
    })
    .unwrap();
    report.retain(task.completion());
    drop(report);
    entered.await.unwrap();
    let completion = workers.completion();
    drop(task);
    drop(workers);
    let held_creator = !completion.is_complete();
    release.send(()).unwrap();
    let original = received.await.unwrap();
    let held_native = !completion.is_complete();
    drop(original);
    completed(&completion).await;
    assert!(held_creator, "T1026 dropped session lost original creator");
    assert!(
        held_native,
        "T1026 late creation lost original native cleanup"
    );
    assert_eq!(sessions.active(), 0);
}
