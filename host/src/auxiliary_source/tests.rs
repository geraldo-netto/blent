//! T1081: original runtime/source ownership, isolated from media and physical APIs.
use super::*;
use blent_config::host_activity::{ActivitySnapshot, Power};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{timeout, Duration};
pub(super) mod fixture;
use fixture::{Factory, Mode};

fn registry(factory: &Arc<Factory>) -> Arc<Registry> {
    Registry::new(
        factory.clone(),
        Settings {
            interval: Duration::from_millis(1),
            ..Default::default()
        },
    )
    .unwrap()
}
async fn until(ready: impl Fn() -> bool) {
    timeout(Duration::from_secs(3), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn opened(registry: &Arc<Registry>) -> Result<Box<dyn ObservationLease>> {
    {
        let registry = registry.clone();
        Opening::start(move || registry.open())?.wait().await
    }
}
async fn observed(
    lease: &dyn ObservationLease,
) -> (watch::Receiver<Publication>, ActivitySnapshot) {
    let mut updates = lease.updates();
    let _ = updates
        .wait_for(|value| matches!(value, Publication::Observed { .. }))
        .await
        .unwrap();
    let value = *updates.borrow();
    let Publication::Observed { snapshot, .. } = value else {
        unreachable!()
    };
    (updates, snapshot)
}
fn isolated_client(registry: Arc<Registry>) -> Result<Box<dyn ObservationLease>> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(opened(&registry))
    })
    .join()
    .unwrap()
}

#[tokio::test]
async fn t1081_three_client_runtimes_share_original_source_until_last_release() {
    let factory = Factory::new(Mode::Ready);
    let registry = registry(&factory);
    let first = isolated_client(registry.clone()).unwrap();
    let second = isolated_client(registry.clone()).unwrap();
    let third = isolated_client(registry.clone()).unwrap();
    let (updates, initial) = observed(&*first).await;
    assert_eq!(factory.count(), 1);
    factory.original(0).event(Power::Suspending);
    until(|| matches!(*updates.borrow(), Publication::Observed { snapshot, .. } if !snapshot.demand.auxiliary)).await;
    factory.original(0).event(Power::Running);
    until(|| matches!(*updates.borrow(), Publication::Observed { snapshot, .. } if snapshot.demand.auxiliary && snapshot.demand.stamp.revision > initial.demand.stamp.revision)).await;
    drop(first);
    drop(second);
    assert!(!factory.original(0).stopped.load(Ordering::Acquire));
    assert_eq!(registry.pool.active(), 1);
    drop(third);
    until(|| registry.pool.active() == 0).await;
    assert!(factory.original(0).stopped.load(Ordering::Acquire));
    assert!(factory.original(0).dropped.load(Ordering::Acquire));
    assert_eq!(*updates.borrow(), Publication::Stopped);
}

#[tokio::test]
async fn t1081_last_release_retains_actual_native_fence_before_source_reuse() {
    let factory = Factory::new(Mode::Ready);
    let release = factory.hold();
    let registry = registry(&factory);
    let first = opened(&registry).await.unwrap();
    let (old, initial) = observed(&*first).await;
    until(|| factory.original(0).entered.load(Ordering::Acquire)).await;
    drop(first);
    until(|| factory.original(0).stopped.load(Ordering::Acquire)).await;
    assert_eq!(registry.pool.active(), 1);
    assert!(opened(&registry).await.is_err());
    let proof = factory
        .original(0)
        .completion
        .lock()
        .unwrap()
        .clone()
        .unwrap();
    assert!(!proof.is_complete());
    release.send(()).unwrap();
    until(|| registry.pool.active() == 0).await;
    assert!(proof.is_complete());
    let next = opened(&registry).await.unwrap();
    let (_, current) = observed(&*next).await;
    assert_ne!(initial.demand.stamp.epoch, current.demand.stamp.epoch);
    factory.original(0).event(Power::Unavailable);
    assert_eq!(*old.borrow(), Publication::Stopped);
    drop(next);
    until(|| registry.pool.active() == 0).await;
}

#[tokio::test]
async fn t1081_last_release_with_borrowed_publication_never_blocks_client() {
    let factory = Factory::new(Mode::Ready);
    let registry = registry(&factory);
    let lease = opened(&registry).await.unwrap();
    let (updates, _) = observed(&*lease).await;
    let borrowed = updates.borrow();
    let (sent, receipt) = std::sync::mpsc::channel();
    let dropper = std::thread::spawn(move || {
        drop(lease);
        let _ = sent.send(());
    });
    let prompt = receipt.recv_timeout(Duration::from_millis(300)).is_ok();
    assert_eq!(registry.pool.active(), 1);
    drop(borrowed);
    dropper.join().unwrap();
    until(|| registry.pool.active() == 0).await;
    assert!(
        prompt,
        "T1081 caller release waited for its own borrowed view"
    );
}

#[tokio::test]
async fn t1081_cancelled_blocking_constructor_disposes_its_exact_late_root() {
    let factory = Factory::new(Mode::Ready);
    let registry = registry(&factory);
    let (entered, entry) = oneshot::channel();
    let (release, gate) = std::sync::mpsc::channel();
    let pending = registry.clone();
    let opening = Opening::start(move || {
        let _ = entered.send(());
        let _ = gate.recv();
        pending.open()
    })
    .unwrap();
    let original = opening.creator.completion();
    entry.await.unwrap();
    drop(opening);
    assert!(!original.is_complete());
    release.send(()).unwrap();
    until(|| original.is_complete()).await;
    until(|| registry.pool.active() == 0).await;
    assert!(factory.count() <= 1);
    let next = opened(&registry).await.unwrap();
    observed(&*next).await;
    drop(next);
    until(|| registry.pool.active() == 0).await;
}

#[tokio::test]
async fn t1081_late_backend_creation_retains_native_ownership_after_last_consumer() {
    let factory = Factory::new(Mode::Late);
    let release = factory.hold();
    let registry = registry(&factory);
    let lease = opened(&registry).await.unwrap();
    let old = lease.updates();
    until(|| factory.count() == 1 && factory.original(0).entered.load(Ordering::Acquire)).await;
    drop(lease);
    until(|| *old.borrow() == Publication::Stopped).await;
    assert_eq!(registry.pool.active(), 1);
    assert!(opened(&registry).await.is_err());
    release.send(()).unwrap();
    until(|| registry.pool.active() == 0).await;
    assert!(factory.original(0).dropped.load(Ordering::Acquire));
    assert_eq!(*old.borrow(), Publication::Stopped);
    let next_ready = factory.hold();
    next_ready.send(()).unwrap();
    let next = opened(&registry).await.unwrap();
    observed(&*next).await;
    drop(next);
    until(|| registry.pool.active() == 0).await;
    assert_eq!(factory.count(), 2);
}

#[tokio::test]
async fn t1081_backend_errors_and_panics_remain_failed_without_legacy_fallback() {
    for mode in [Mode::Failed, Mode::Panicked] {
        let factory = Factory::new(mode);
        let registry = registry(&factory);
        let lease = opened(&registry).await.unwrap();
        let view = lease.updates();
        until(|| *view.borrow() == Publication::Failed).await;
        assert!(opened(&registry).await.is_err());
        assert_eq!(factory.count(), 0);
        drop(lease);
        until(|| registry.pool.active() == 0).await;
    }
}

#[tokio::test]
async fn t1081_constructor_native_exhaustion_joins_failed_worker_without_replacing_original() {
    let factory = Factory::new(Mode::Ready);
    let capacity =
        blent_config::retirement_limits::configured("BLENT_ACTIVITY_LIMIT", 1, 4).unwrap();
    let mut holders = Vec::new();
    for _ in 0..capacity {
        let owner = registry(&factory);
        let lease = opened(&owner).await.unwrap();
        observed(&*lease).await;
        holders.push((owner, lease));
    }
    let other = registry(&factory);
    assert!(opened(&other).await.is_err());
    until(|| other.pool.active() == 0).await;
    assert_eq!(factory.count(), capacity);
    assert!(!factory.original(0).stopped.load(Ordering::Acquire));
    for (owner, lease) in holders {
        drop(lease);
        until(|| owner.pool.active() == 0).await;
    }
    let next = opened(&other).await.unwrap();
    observed(&*next).await;
    drop(next);
    until(|| other.pool.active() == 0).await;
}

#[test]
fn t1081_invalid_settings_and_cached_failures_never_retry_initialization() {
    let factory = Factory::new(Mode::Ready);
    for settings in [
        Settings {
            startup_deadline: Duration::ZERO,
            ..Default::default()
        },
        Settings {
            interval: Duration::ZERO,
            ..Default::default()
        },
    ] {
        assert!(Registry::new(factory.clone(), settings).is_err());
    }
    let cell = OnceLock::new();
    let calls = AtomicUsize::new(0);
    assert!(cached(&cell, || {
        calls.fetch_add(1, Ordering::SeqCst);
        anyhow::bail!("T1081 unavailable")
    })
    .is_err());
    assert!(cached(&cell, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Registry::new(factory.clone(), Settings::default())
    })
    .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(factory.count(), 0);
}

#[tokio::test]
async fn t1081_private_public_adapter_is_explicitly_unsupported_without_logind() {
    if !crate::activity::private_bus_test("auxiliary_source::tests::t1081_private_public_adapter_is_explicitly_unsupported_without_logind") { return; }
    let lease = acquire().await.unwrap();
    let registry = shared().unwrap();
    let (updates, _) = observed(&*lease).await;
    let value = *updates.borrow();
    let Publication::Observed { capabilities, .. } = value else {
        unreachable!()
    };
    assert_eq!(capabilities, Default::default());
    drop(lease);
    until(|| registry.pool.active() == 0).await;
}

#[tokio::test]
async fn t1081_concurrent_admission_and_all_six_release_orders_keep_one_source() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        concurrent_release_order(order).await;
    }
}
async fn concurrent_release_order(order: [usize; 3]) {
    let factory = Factory::new(Mode::Ready);
    let registry = registry(&factory);
    let (first, second, third) =
        tokio::join!(opened(&registry), opened(&registry), opened(&registry));
    let mut leases = [
        Some(first.unwrap()),
        Some(second.unwrap()),
        Some(third.unwrap()),
    ];
    observed(&**leases[0].as_ref().unwrap()).await;
    assert_eq!(factory.count(), 1);
    for index in &order[..2] {
        drop(leases[*index].take());
        assert!(!factory.original(0).stopped.load(Ordering::Acquire));
    }
    drop(leases[order[2]].take());
    until(|| registry.pool.active() == 0).await;
    assert!(factory.original(0).stopped.load(Ordering::Acquire));
}

#[tokio::test]
async fn t1081_owner_lost_before_startup_receipt_still_joins_original_worker() {
    let factory = Factory::new(Mode::Ready);
    let registry = registry(&factory);
    let (worker, response) =
        worker::start(&registry.pool, factory.clone(), registry.settings).unwrap();
    drop(response);
    drop(worker);
    until(|| registry.pool.active() == 0).await;
    assert!(factory.count() <= 1);
}

#[test]
fn t1081_worker_unwind_closes_original_publication_before_runtime_destruction() {
    let factory = Factory::new(Mode::Ready);
    // Deliberately faulty runtime: the retirement timer panics after exposure.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let (_stop, stopped) = watch::channel(true);
    let (reply, response) = std::sync::mpsc::sync_channel(1);
    worker::work(runtime, factory, Settings::default(), stopped, reply);
    let original = response.recv().unwrap().unwrap();
    assert_eq!(*original.subscribe().borrow(), Publication::Stopped);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !original.retired() {
        assert!(
            std::time::Instant::now() < deadline,
            "T1081 original unwind cleanup"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
