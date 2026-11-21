use super::*;
use blent_config::retirement::{Completion, Pool};
use std::sync::Arc;

async fn completed(proof: Completion) {
    tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        while !proof.is_complete() {
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "T1410 original session did not settle"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    })
    .await
    .unwrap();
}

/// Inject a delayed native completion through the existing supervisor contract.
/// No devices are created: the test owns one independent native admission.
async fn hold_native(
    monitor: &mut Monitor<super::super::tests::Fake>,
    proof: Completion,
) -> Completion {
    let active = monitor.slots[0].as_mut().unwrap();
    let mut workers = crate::session_supervision::Workers::empty(
        crate::session_supervision::Workers::reserve().unwrap(),
    );
    workers.capture_lifetime().retain(proof);
    let runtime = Runtime::from_workers(
        0,
        super::super::tests::fixture().1.attachment,
        Arc::new(tokio::sync::Notify::new()),
        watch::channel(false).0,
        workers,
        monitor.ports[0],
    );
    let completion = runtime.retirement.clone();
    let original = std::mem::replace(&mut active.runtime, runtime);
    completed(original.stop().await).await;
    let serial = active.connection.selected().unwrap();
    active.connection.disconnect(&monitor.adb).await.unwrap();
    active.connection = Connection::new(active.runtime.tablet_tx.clone(), monitor.ports[0], false)
        .unwrap()
        .with_native_retirement(completion.clone())
        .unwrap();
    active
        .connection
        .connect(&monitor.adb, &serial)
        .await
        .unwrap();
    completion
}

#[tokio::test]
async fn t1410_pending_native_session_excludes_its_slot_and_serial() {
    let (_, mut monitor, _stop, _) = super::super::tests::monitor_fixture().await;
    monitor.poll().await;
    let native = Pool::new(1).unwrap().reserve().unwrap();
    hold_native(&mut monitor, native.completion()).await;
    monitor.retire(0).await;
    let excluded = !monitor.available("USB");
    monitor.update(0, &["B".into()]).await.unwrap();
    let reserved = monitor.slots[0].is_none();
    drop(native);
    monitor.shutdown().await.unwrap();
    assert!(
        excluded,
        "T1410 original native session lost serial reservation"
    );
    assert!(reserved, "T1410 slot reused before native completion");
}

#[tokio::test]
async fn t1410_retirement_preserves_sibling_progress_and_original_slot_recovery() {
    let (adb, mut monitor, _stop, ports) = super::super::tests::monitor_fixture().await;
    super::super::tests::set_inventory(&adb, "List of devices attached\nUSB\tdevice\nB\tdevice\n");
    monitor.config.max_tablets = 2;
    monitor.ports = blent_config::slot_ports(ports.0, ports.1, 2).unwrap();
    monitor.slots.push(None);
    monitor.poll().await;
    let native = Pool::new(1).unwrap().reserve().unwrap();
    let proof = hold_native(&mut monitor, native.completion()).await;
    monitor.retire(0).await;
    for _ in 0..16 {
        monitor
            .update(0, &["USB".into(), "B".into()])
            .await
            .unwrap();
        assert!(monitor.slots[0].is_none());
        assert_eq!(
            monitor.retiring.len(),
            1,
            "T1410 retirement exceeded one record per slot"
        );
    }
    monitor
        .update(1, &["USB".into(), "B".into()])
        .await
        .unwrap();
    assert_eq!(
        monitor.slots[1]
            .as_ref()
            .unwrap()
            .connection
            .serial()
            .as_deref(),
        Some("B")
    );
    drop(native);
    completed(proof).await;
    monitor.settle_sessions();
    monitor.retry_routes(None).await;
    monitor
        .update(0, &["USB".into(), "B".into()])
        .await
        .unwrap();
    assert_eq!(monitor.sessions().len(), 2);
    monitor.shutdown().await.unwrap();
}

#[tokio::test]
async fn t1410_cancelled_stop_retains_reservation_before_first_await() {
    let (_, mut monitor, _stop, _) = super::super::tests::monitor_fixture().await;
    monitor.poll().await;
    let native = Pool::new(1).unwrap().reserve().unwrap();
    hold_native(&mut monitor, native.completion()).await;
    let mut stopping = Box::pin(monitor.retire(0));
    assert!(futures_util::poll!(stopping.as_mut()).is_pending());
    drop(stopping);
    let retained = !monitor.available("USB") && monitor.retiring.len() == 1;
    monitor.update(0, &["B".into()]).await.unwrap();
    let reserved = monitor.slots[0].is_none();
    drop(native);
    monitor.shutdown().await.unwrap();
    assert!(retained, "T1410 cancellation forgot original session");
    assert!(reserved, "T1410 cancellation allowed slot reuse");
}

#[tokio::test]
async fn t1410_shutdown_waits_for_original_native_completion_and_can_resume() {
    let (_, mut monitor, _stop, _) = super::super::tests::monitor_fixture().await;
    monitor.poll().await;
    let native = Pool::new(1).unwrap().reserve().unwrap();
    hold_native(&mut monitor, native.completion()).await;
    let mut stopping = Box::pin(monitor.shutdown());
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut stopping)
            .await
            .is_err()
    );
    drop(stopping);
    assert_eq!(monitor.retiring.len(), 1);
    drop(native);
    tokio::time::timeout(Duration::from_secs(3), monitor.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert!(monitor.retiring.is_empty());
    assert!(monitor.pending.is_empty());
}
