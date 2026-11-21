//! T1460: a consumed replacement must settle even when original cleanup fails.
use super::*;

#[tokio::test]
async fn t1460_failed_cleanup_rejects_replacement_without_restarting() {
    let (first, _) = Request::new(Session::new(CameraProfile::default()));
    let (command, commands) = watch::channel(Some(first));
    let (sender, mut state) = watch::channel(State::Stopped);
    let report = Report::new(sender, watch::channel(None).0);
    let native = blent_config::retirement::Pool::new(1)
        .unwrap()
        .reserve()
        .unwrap();
    let proof = native.completion();
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let calls = count.clone();
    let task = tokio::spawn(supervise(commands, report, move |_, mut stop, report| {
        report
            .reserve_native(native::Resource::Connection)
            .unwrap()
            .bind(proof.clone());
        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            cancelled(&mut stop).await;
            anyhow::bail!("T1460 original cleanup failed")
        }
    }));
    state
        .wait_for(|value| *value == State::Starting)
        .await
        .unwrap();
    let (next, receipt) = Request::new(Session::new(CameraProfile::default()));
    command.send(Some(next)).unwrap();
    state
        .wait_for(|value| *value == State::Stopping)
        .await
        .unwrap();
    assert!(
        receipt.is_pending(),
        "T1460 rejected before native settlement"
    );
    drop(native);
    state
        .wait_for(|value| matches!(value, State::Failed(_)))
        .await
        .unwrap();
    let pending_at_failure = receipt.is_pending();
    drop(command);
    task.await.unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(
        !pending_at_failure,
        "T1460 replacement stuck at Starting after failed cleanup"
    );
    assert!(
        !receipt.is_pending(),
        "T1460 caller loss left a pending receipt"
    );
}
