//! T1418: a replacement must not forget an already observed native capability.
use super::*;
use blent_config::host_activity::{Publication, SourceCreation, SourceFactory};
use blent_config::retirement::Completion;
use std::sync::atomic::AtomicUsize;

struct Counted {
    native: Arc<dyn SourceFactory>,
    calls: AtomicUsize,
    proofs: Mutex<Vec<Completion>>,
    held: Mutex<Option<Completion>>,
}
impl SourceFactory for Counted {
    fn create(&self, settings: Settings, lifetime: SourceLifetime) -> SourceCreation {
        assert!(
            self.proofs
                .lock()
                .unwrap()
                .iter()
                .all(Completion::is_complete),
            "T1418 every original attempt must settle before replacement"
        );
        self.proofs.lock().unwrap().push(lifetime.completion());
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(held) = self.held.lock().unwrap().take() {
            lifetime.retain(held);
        }
        self.native.create(settings, lifetime)
    }
}
fn start(native: Arc<dyn SourceFactory>) -> (Arc<Counted>, crate::activity_follow::Following) {
    start_held(native, None)
}
fn start_held(
    native: Arc<dyn SourceFactory>,
    held: Option<Completion>,
) -> (Arc<Counted>, crate::activity_follow::Following) {
    let factory = Arc::new(Counted {
        native,
        calls: AtomicUsize::new(0),
        proofs: Mutex::new(Vec::new()),
        held: Mutex::new(held),
    });
    let following = crate::activity_follow::Following::start(
        factory.clone(),
        crate::activity_follow::Settings {
            source: Settings {
                deadline: Duration::from_millis(30),
                ..Default::default()
            },
            interval: Duration::from_millis(1),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    (factory, following)
}
async fn attempts(factory: &Counted, minimum: usize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while factory.calls.load(Ordering::Acquire) < minimum {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn conservative(reader: &mut tokio::sync::watch::Receiver<Publication>) {
    let deadline = tokio::time::sleep(Duration::from_millis(180));
    tokio::pin!(deadline);
    loop {
        if let Publication::Observed { snapshot, .. } = *reader.borrow_and_update() {
            assert!(
                !snapshot.demand.display,
                "T1418 missing known observer must deny display"
            );
        }
        tokio::select! {
            _ = &mut deadline => return,
            value = reader.changed() => value.unwrap(),
        }
    }
}
async fn finish(factory: &Counted, following: crate::activity_follow::Following) {
    recovery::finish(following).await;
    assert!(factory
        .proofs
        .lock()
        .unwrap()
        .iter()
        .all(Completion::is_complete));
    let calls = factory.calls.load(Ordering::Acquire);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        calls,
        "T1418 stopped recovery must not recreate"
    );
}
async fn delayed(native: Arc<dyn SourceFactory>, name: &str) {
    let fixture = Fixture::new().await;
    let (factory, following) = start(native);
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |s| s.demand.display).await;
    fixture.server.release_name(name).await.unwrap();
    followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    attempts(&factory, 3).await;
    if name == LOGIN {
        fixture.values.sleep.store(true, Ordering::Release);
    } else {
        fixture.values.display.store(3, Ordering::Release);
    }
    let replacement = recovery::replacement(&fixture, name).await;
    let fresh = followed(&mut reader, |s| {
        s.demand.stamp.epoch > first.demand.stamp.epoch
    })
    .await;
    assert!(
        !fresh.demand.display,
        "T1418 fresh native state, not former awake state"
    );
    fixture.values.sleep.store(false, Ordering::Release);
    fixture.values.display.store(0, Ordering::Release);
    if name == LOGIN {
        replacement
            .emit_signal(None::<&str>, PATH, MANAGER, "PrepareForSleep", &(false,))
            .await
            .unwrap();
    } else {
        // Reacquisition must continue following the replacement owner's signals.
        let values = std::collections::HashMap::from([("PowerSaveMode", OwnedValue::from(0i32))]);
        replacement
            .emit_signal(
                None::<&str>,
                DISPLAY_PATH,
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
                &(DISPLAY, values, Vec::<String>::new()),
            )
            .await
            .unwrap();
    }
    followed(&mut reader, |s| s.demand.display).await;
    finish(&factory, following).await;
    replacement.close().await.unwrap();
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1418_delayed_power_owner_recovers_with_fresh_state() {
    if !private("activity::tests::continuity::t1418_delayed_power_owner_recovers_with_fresh_state")
    {
        return;
    }
    delayed(Arc::new(PowerFactory::default()), LOGIN).await;
}
#[tokio::test]
async fn t1418_delayed_display_owner_recovers_with_fresh_state() {
    if !private(
        "activity::tests::continuity::t1418_delayed_display_owner_recovers_with_fresh_state",
    ) {
        return;
    }
    delayed(Arc::new(Factory::default()), DISPLAY).await;
}
#[tokio::test]
async fn t1418_prolonged_absence_remains_conservative_and_stop_settles() {
    if !private("activity::tests::continuity::t1418_prolonged_absence_remains_conservative_and_stop_settles") { return; }
    let fixture = Fixture::new().await;
    let (factory, following) = start(Arc::new(PowerFactory::default()));
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.demand.display).await;
    fixture.server.release_name(LOGIN).await.unwrap();
    followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    attempts(&factory, 3).await;
    assert_eq!(*reader.borrow(), Publication::Failed);
    finish(&factory, following).await;
    assert_eq!(*reader.borrow(), Publication::Stopped);
    fixture.server.close().await.unwrap();
}
async fn unsupported(native: Arc<dyn SourceFactory>) {
    let (factory, following) = start(native);
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |s| s.demand.display).await;
    assert!(first.demand.auxiliary);
    assert!(
        matches!(*reader.borrow(), Publication::Observed { capabilities, .. }
        if capabilities == Capabilities::default())
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
    finish(&factory, following).await;
}
#[tokio::test]
async fn t1418_initially_unsupported_stays_explicit_without_retry() {
    if !private(
        "activity::tests::continuity::t1418_initially_unsupported_stays_explicit_without_retry",
    ) {
        return;
    }
    unsupported(Arc::new(PowerFactory::default())).await;
    unsupported(Arc::new(Factory::default())).await;
}

fn capabilities(bits: u8) -> Capabilities {
    Capabilities {
        power: bits & 1 != 0,
        lock: bits & 2 != 0,
        display: bits & 4 != 0,
    }
}

#[test]
fn t1418_all_capability_subsets_reject_loss_without_forgetting_admission() {
    for original in 0..8 {
        for candidate in 0..8 {
            let history = super::super::continuity::Continuity::default();
            history.accept(capabilities(original)).unwrap();
            let outcome = history.clone().accept(capabilities(candidate));
            let lost = capabilities(original & !candidate);
            let expected = lost == Capabilities::default();
            assert_eq!(outcome.is_ok(), expected);
            let retained = original | candidate;
            // Rejection cannot erase history; accepted gains cannot be forgotten.
            let current = if expected { candidate } else { original };
            assert_eq!(
                history.accept(Capabilities::default()).is_ok(),
                current == 0
            );
            history.accept(capabilities(retained)).unwrap();
        }
    }
}

#[tokio::test]
async fn t1418_rejected_partial_source_retains_native_debt_through_stop() {
    if !private("activity::tests::continuity::t1418_rejected_partial_source_retains_native_debt_through_stop") { return; }
    let fixture = Fixture::new().await;
    let (factory, following) = start(Arc::new(Factory::default()));
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.demand.display).await;
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    *factory.held.lock().unwrap() = Some(held.completion());
    fixture.server.release_name(DISPLAY).await.unwrap();
    tokio::time::timeout(
        Duration::from_secs(3),
        reader.wait_for(|p| *p == Publication::Failed),
    )
    .await
    .unwrap()
    .unwrap();
    conservative(&mut reader).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        2,
        "T1418 no retry before rejected native settlement"
    );
    following.stop();
    assert!(
        !following.retired(),
        "T1418 stop cannot erase original native debt"
    );
    drop(held);
    finish(&factory, following).await;
    fixture.server.close().await.unwrap();
}

async fn invalidated(fixture: &Fixture, display: bool) {
    if display {
        fixture.values.fail_display.store(true, Ordering::Release);
        fixture
            .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", None)
            .await;
    } else {
        fixture.values.fail_lock.store(true, Ordering::Release);
        fixture
            .changed(SESSION_PATH, SESSION, "LockedHint", None)
            .await;
    }
}
async fn refresh_failure(display: bool) {
    let fixture = Fixture::new().await;
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, following) = start_held(Arc::new(Factory::default()), Some(held.completion()));
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |s| s.demand.display).await;
    invalidated(&fixture, display).await;
    followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        1,
        "T1427 original native debt fences retry"
    );
    assert!(!factory.proofs.lock().unwrap()[0].is_complete());
    fixture.values.fail_display.store(false, Ordering::Release);
    fixture.values.fail_lock.store(false, Ordering::Release);
    drop(held);
    let fresh = followed(&mut reader, |s| {
        s.demand.display && s.demand.stamp.epoch > first.demand.stamp.epoch
    })
    .await;
    assert!(!fresh.source_lost);
    finish(&factory, following).await;
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1427_failed_lock_and_display_refresh_recover_after_original_settlement() {
    if !private("activity::tests::continuity::t1427_failed_lock_and_display_refresh_recover_after_original_settlement") { return; }
    for display in [false, true] {
        refresh_failure(display).await;
    }
}
#[tokio::test]
async fn t1427_prolonged_refresh_failure_stays_conservative_and_stop_retires() {
    if !private("activity::tests::continuity::t1427_prolonged_refresh_failure_stays_conservative_and_stop_retires") { return; }
    let fixture = Fixture::new().await;
    let (factory, following) = start(Arc::new(Factory::default()));
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.demand.display).await;
    invalidated(&fixture, true).await;
    followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    attempts(&factory, 3).await;
    assert!(
        matches!(
            *reader.borrow(),
            Publication::Observed { capabilities, snapshot }
                if capabilities.display && snapshot.source_lost && !snapshot.demand.display
        ),
        "T1436 known failed display retains authority while retrying"
    );
    finish(&factory, following).await;
    assert_eq!(*reader.borrow(), Publication::Stopped);
    fixture.server.close().await.unwrap();
}

async fn initial_failure(field: usize) {
    let fixture = Fixture::new().await;
    let failures = [
        &fixture.values.fail_sleep,
        &fixture.values.fail_shutdown,
        &fixture.values.fail_lock,
    ];
    let failing = failures[field];
    failing.store(true, Ordering::Release);
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, following) =
        start_held(Arc::new(PowerFactory::default()), Some(held.completion()));
    let mut reader = following.subscribe();
    let lost = followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        1,
        "T1429 failed initial native debt fences retry"
    );
    assert!(!factory.proofs.lock().unwrap()[0].is_complete());
    drop(held);
    attempts(&factory, 3).await;
    conservative(&mut reader).await;
    failing.store(false, Ordering::Release);
    let fresh = followed(&mut reader, |s| s.demand.display).await;
    assert!(fresh.demand.stamp.epoch > lost.demand.stamp.epoch);
    assert!(fresh.demand.auxiliary);
    finish(&factory, following).await;
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1429_initial_power_and_lock_query_failures_retry_after_original_settlement() {
    if !private("activity::tests::continuity::t1429_initial_power_and_lock_query_failures_retry_after_original_settlement") { return; }
    for field in 0..3 {
        initial_failure(field).await;
    }
}
#[tokio::test]
async fn t1429_stop_during_failed_initial_snapshot_keeps_original_native_debt() {
    if !private("activity::tests::continuity::t1429_stop_during_failed_initial_snapshot_keeps_original_native_debt") { return; }
    let fixture = Fixture::new().await;
    fixture.values.fail_sleep.store(true, Ordering::Release);
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, following) =
        start_held(Arc::new(PowerFactory::default()), Some(held.completion()));
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.source_lost).await;
    following.stop();
    assert!(
        !following.retired(),
        "T1429 stop cannot erase initial query native debt"
    );
    drop(held);
    finish(&factory, following).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1429_explicit_unsupported_property_stays_unknown_without_retry() {
    if !private("activity::tests::continuity::t1429_explicit_unsupported_property_stays_unknown_without_retry") { return; }
    let fixture = Fixture::new().await;
    fixture.values.missing_sleep.store(true, Ordering::Release);
    let (factory, following) = start(Arc::new(PowerFactory::default()));
    let mut reader = following.subscribe();
    let snapshot = followed(&mut reader, |s| !s.demand.display).await;
    assert!(!snapshot.source_lost);
    assert!(!snapshot.demand.auxiliary);
    conservative(&mut reader).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
    finish(&factory, following).await;
    fixture.server.close().await.unwrap();
}

async fn initial_display_failure(kind: usize) {
    let fixture = Fixture::new().await;
    fixture
        .values
        .display
        .store(if kind == 2 { 4 } else { 3 }, Ordering::Release);
    fixture
        .values
        .fail_display
        .store(kind == 0, Ordering::Release);
    fixture
        .values
        .display_delay_ms
        .store(if kind == 1 { 100 } else { 0 }, Ordering::Release);
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, following) = start_held(Arc::new(Factory::default()), Some(held.completion()));
    let mut reader = following.subscribe();
    let lost = followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        1,
        "T1436 known display failure retains original debt"
    );
    assert!(!factory.proofs.lock().unwrap()[0].is_complete());
    fixture.values.fail_display.store(false, Ordering::Release);
    fixture.values.display_delay_ms.store(0, Ordering::Release);
    fixture.values.display.store(3, Ordering::Release);
    drop(held);
    let fresh = followed(&mut reader, |s| {
        s.demand.stamp.epoch > lost.demand.stamp.epoch && !s.source_lost
    })
    .await;
    assert!(
        !fresh.demand.display,
        "T1436 fresh display-off snapshot must deny demand"
    );
    assert!(
        matches!(*reader.borrow(), Publication::Observed { capabilities, .. } if capabilities.display)
    );
    fixture.values.display.store(0, Ordering::Release);
    fixture
        .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", Some(0i32.into()))
        .await;
    followed(&mut reader, |s| s.demand.display).await;
    finish(&factory, following).await;
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1436_initial_display_read_error_and_timeout_recover_after_original_settlement() {
    if !private("activity::tests::continuity::t1436_initial_display_read_error_and_timeout_recover_after_original_settlement") { return; }
    for kind in 0..3 {
        initial_display_failure(kind).await;
    }
}
#[tokio::test]
async fn t1436_initial_display_failure_stop_keeps_original_debt() {
    if !private(
        "activity::tests::continuity::t1436_initial_display_failure_stop_keeps_original_debt",
    ) {
        return;
    }
    let fixture = Fixture::new().await;
    fixture.values.fail_display.store(true, Ordering::Release);
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, following) = start_held(Arc::new(Factory::default()), Some(held.completion()));
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.source_lost).await;
    conservative(&mut reader).await;
    following.stop();
    assert!(!following.retired());
    drop(held);
    finish(&factory, following).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
    fixture.server.close().await.unwrap();
}
#[tokio::test]
async fn t1436_explicit_unsupported_initial_display_stays_non_retrying() {
    if !private("activity::tests::continuity::t1436_explicit_unsupported_initial_display_stays_non_retrying") { return; }
    let fixture = Fixture::new().await;
    for missing_property in [false, true] {
        fixture
            .values
            .missing_display
            .store(missing_property, Ordering::Release);
        fixture.values.display.store(-1, Ordering::Release);
        let (factory, following) = start(Arc::new(Factory::default()));
        let mut reader = following.subscribe();
        let first = followed(&mut reader, |s| s.demand.display).await;
        assert!(!first.source_lost);
        assert!(
            matches!(*reader.borrow(), Publication::Observed { capabilities, .. } if !capabilities.display)
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(factory.calls.load(Ordering::Acquire), 1);
        finish(&factory, following).await;
    }
    fixture.server.close().await.unwrap();
}
