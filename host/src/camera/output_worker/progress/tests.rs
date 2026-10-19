use super::*;
use std::sync::mpsc;

async fn held() -> (Task, mpsc::SyncSender<()>) {
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, blocked) = mpsc::sync_channel(1);
    let native = Task::spawn_blocking(move || {
        let _ = entered.send(());
        blocked.recv()?;
        Ok(())
    })
    .unwrap();
    ready.await.unwrap();
    (native, release)
}
fn observer(mut native: Task, mut observed: watch::Receiver<Option<Pending>>) -> Task {
    Task::spawn(async move { monitor(&mut native, &mut observed).await }).unwrap()
}

#[tokio::test(start_paused = true)]
async fn t1148_idle_or_closed_progress_never_implies_native_completion() {
    for closed in [false, true] {
        let (native, release) = held().await;
        let completion = native.completion();
        let (progress, observed) = watch::channel(None);
        let controller = observer(native, observed);
        let retained = if closed {
            drop(progress);
            None
        } else {
            Some(progress)
        };
        tokio::time::advance(Duration::from_secs(3)).await;
        tokio::task::yield_now().await;
        let pending = !controller.is_finished() && !completion.is_complete();
        release.send(()).unwrap();
        controller.await.unwrap().unwrap();
        drop(retained);
        assert!(pending);
        assert!(completion.is_complete());
    }
}

#[tokio::test(start_paused = true)]
async fn t1148_replaced_or_completed_operation_cancels_the_original_deadline() {
    let (native, release) = held().await;
    let (mut progress, observed) = watch::channel(None);
    let initial = Span::begin(&mut progress, Kind::Write);
    let controller = observer(native, observed);
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(1900)).await;
    drop(initial);
    let next = Span::begin(&mut progress, Kind::Configuration);
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(200)).await;
    tokio::task::yield_now().await;
    let pending = !controller.is_finished();
    drop(next);
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    let idle = !controller.is_finished();
    release.send(()).unwrap();
    controller.await.unwrap().unwrap();
    assert!(pending && idle);
}

#[tokio::test]
async fn t1148_native_panic_is_reported_after_actual_task_retirement() {
    let native = Task::spawn_blocking(|| panic!("T1148 native operation fault")).unwrap();
    let completion = native.completion();
    let (progress, observed) = watch::channel(None);
    drop(progress);
    let result = observer(native, observed).await.unwrap();
    assert!(result.is_err());
    assert!(completion.is_complete());
}

#[tokio::test(start_paused = true)]
async fn t1148_expired_snapshot_cannot_alarm_a_cleared_or_different_operation() {
    let original = Pending {
        kind: Kind::Write,
        started: Instant::now(),
    };
    let expired = expiration(Some(original)).await.unwrap();
    for current in [
        None,
        Some(Pending {
            kind: Kind::Configuration,
            started: original.started,
        }),
        Some(Pending {
            kind: Kind::Write,
            started: Instant::now(),
        }),
    ] {
        assert!(reject_current(Expired(expired.0), current).is_ok());
    }
}
