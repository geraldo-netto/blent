//! T1125: logical outcomes and stale callbacks never retire original resources.
use super::*;
use blent_config::{camera::CameraBackend, retirement::Pool};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

mod owned_future;
mod scopes;

fn until(condition: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(
            Instant::now() < end,
            "T1125 fixture observation did not settle"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn t1125_success_failure_and_panic_cannot_replace_unretired_original() {
    for outcome in 0..3 {
        replacement(outcome);
    }
}

struct Fixture {
    controller: Controller,
    original: blent_config::retirement::Admission,
    calls: Arc<AtomicUsize>,
    reports: Arc<Mutex<Vec<Report>>>,
}
impl Fixture {
    fn new(outcome: u8, workers: crate::control_workers::WorkerPool) -> Self {
        let original = Pool::new(1).unwrap().reserve().unwrap();
        let proof = original.completion();
        let calls = Arc::new(AtomicUsize::new(0));
        let started = calls.clone();
        let reports = Arc::new(Mutex::new(Vec::new()));
        let observed = reports.clone();
        let controller = Controller::with_pool(workers, move |_, _, report| {
            let call = started.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                report
                    .reserve_native(native::Resource::Connection)
                    .unwrap()
                    .bind(proof.clone());
            }
            observed.lock().unwrap().push(report);
            std::future::ready(result(outcome))
        })
        .unwrap();
        controller.start(CameraProfile::default()).unwrap();
        until(|| calls.load(Ordering::SeqCst) == 1);
        until(|| {
            matches!(
                controller.state(),
                State::Stopped | State::Stopping | State::Failed(_)
            )
        });
        Self {
            controller,
            original,
            calls,
            reports,
        }
    }
}
fn result(outcome: u8) -> Result<()> {
    match outcome {
        0 => Ok(()),
        1 => anyhow::bail!("T1125 original logical failure"),
        _ => panic!("T1125 original factory panic"),
    }
}

fn replacement(outcome: u8) {
    let fixture = Fixture::new(outcome, crate::control_workers::WorkerPool::new(1).unwrap());
    fixture.controller.start(CameraProfile::default()).unwrap();
    until(|| {
        fixture.calls.load(Ordering::SeqCst) > 1 || fixture.controller.state() == State::Stopping
    });
    let early_calls = fixture.calls.load(Ordering::SeqCst);
    let old_report = fixture.reports.lock().unwrap()[0].clone();
    old_report.update(State::Streaming);
    let old_ignored = fixture.controller.state() != State::Streaming;
    drop(fixture.original);
    until(|| fixture.calls.load(Ordering::SeqCst) >= 2);
    let completion = fixture.controller.shutdown();
    until(|| completion.is_complete());
    assert_eq!(
        early_calls, 1,
        "T1125 replaced original before native retirement"
    );
    assert!(
        old_ignored,
        "T1125 stale original callback changed current workflow"
    );
}

#[test]
fn t1125_owner_loss_keeps_native_fence_without_blocking_sibling() {
    let workers = crate::control_workers::WorkerPool::new(2).unwrap();
    let fixture = Fixture::new(0, workers.clone());
    let sibling_calls = Arc::new(AtomicUsize::new(0));
    let observed = sibling_calls.clone();
    let sibling = Controller::with_pool(workers, move |_, _, _| {
        observed.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    })
    .unwrap();
    sibling.start(CameraProfile::default()).unwrap();
    until(|| sibling_calls.load(Ordering::SeqCst) == 1);
    let original_completion = fixture.controller.shutdown();
    let independent_completion = sibling.shutdown();
    until(|| independent_completion.is_complete());
    let retained = !original_completion.is_complete();
    drop(fixture.original);
    until(|| original_completion.is_complete());
    assert!(retained, "T1125 worker exit released unretired original");
}

#[test]
fn t1125_group_bounds_reserve_before_effects_and_share_original_scopes() {
    for (kind, capacity) in [
        (native::Resource::Connection, 1),
        (native::Resource::Devices, 2),
        (native::Resource::Producer, 4),
        (native::Resource::Decoder, 1),
    ] {
        group(kind, capacity);
    }
}
fn group(kind: native::Resource, capacity: usize) {
    let report = Report::new(watch::channel(State::Stopped).0, watch::channel(None).0);
    let scope = crate::report_scope::Owner::default();
    scope.begin();
    let scoped = report.scoped(scope.permit().unwrap());
    let mut slots = Vec::new();
    for _ in 0..capacity {
        slots.push(scoped.reserve_native(kind).unwrap());
    }
    assert!(report.reserve_native(kind).is_err());
    assert!(!report.native_retired());
    drop(slots);
    assert!(report.native_retired());
    let pool = Pool::new(1).unwrap();
    let owner = pool.reserve().unwrap();
    report
        .reserve_native(kind)
        .unwrap()
        .bind(owner.completion());
    assert!(!scoped.native_retired());
    drop(owner);
    assert!(report.native_retired());
    scope.revoke();
    assert!(
        scoped.reserve_native(kind).is_err(),
        "T1125 stale report admitted native creation"
    );
    assert!(report.native_retired());
}
