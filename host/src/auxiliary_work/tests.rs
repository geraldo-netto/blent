//! T1080: bounded independent resource obligations; no OS/device operations.
use super::*;
use blent_config::host_activity::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::time::{timeout, Duration};

struct Source {
    owner: watch::Sender<Publication>,
    feed: Feed,
    revision: u64,
    capabilities: Capabilities,
}
impl Source {
    fn new(power: bool) -> Self {
        let capabilities = Capabilities {
            power,
            lock: true,
            display: true,
        };
        let (owner, _) = watch::channel(Publication::Pending);
        let mut source = Self {
            owner,
            feed: Feed::new(1, capabilities, Settings::default()).unwrap(),
            revision: 0,
            capabilities,
        };
        source.emit(Power::Running, false, true);
        source
    }
    fn emit(&mut self, power: Power, locked: bool, display: bool) {
        self.revision += 1;
        self.feed
            .observe(Event {
                epoch: 1,
                revision: self.revision,
                state: HostState {
                    power: self.capabilities.power.then_some(power),
                    locked: Some(locked),
                    display_on: Some(display),
                },
            })
            .unwrap();
        self.owner.send_replace(Publication::Observed {
            capabilities: self.capabilities,
            snapshot: self.feed.snapshot(true, false),
        });
    }
}

struct Original {
    stopped: AtomicBool,
    settled: AtomicBool,
    release: watch::Sender<bool>,
}
struct Native {
    originals: Mutex<Vec<Arc<Original>>>,
    auto_retire: bool,
}
impl Native {
    fn new(auto_retire: bool) -> Arc<Self> {
        Arc::new(Self {
            originals: Mutex::new(Vec::new()),
            auto_retire,
        })
    }
    fn original(&self, index: usize) -> Arc<Original> {
        self.originals.lock().unwrap()[index].clone()
    }
    fn count(&self) -> usize {
        self.originals.lock().unwrap().len()
    }
    fn start(
        &self,
        mut stop: watch::Receiver<bool>,
    ) -> impl Future<Output = Result<()>> + Send + 'static {
        let original = Arc::new(Original {
            stopped: AtomicBool::new(false),
            settled: AtomicBool::new(false),
            release: watch::channel(self.auto_retire).0,
        });
        {
            let mut all = self.originals.lock().unwrap();
            assert!(all.len() < 16);
            assert!(all
                .iter()
                .all(|prior| prior.settled.load(Ordering::Acquire)));
            all.push(original.clone());
        }
        async move {
            let _ = stop.wait_for(|value| *value).await;
            original.stopped.store(true, Ordering::Release);
            Ok(()) // Logical completion deliberately precedes independent native proof.
        }
    }
    fn settle(&self) -> impl Future<Output = ()> + Send + 'static {
        let original = self.original(self.count() - 1);
        async move {
            let mut released = original.release.subscribe();
            released.wait_for(|value| *value).await.unwrap();
            assert!(!original.settled.swap(true, Ordering::AcqRel));
        }
    }
    fn release_all(&self) {
        for original in self.originals.lock().unwrap().iter() {
            original.release.send_replace(true);
        }
    }
    fn conserved(&self) {
        let all = self.originals.lock().unwrap();
        assert!(all
            .iter()
            .all(|original| original.settled.load(Ordering::Acquire)));
        assert!(all
            .iter()
            .all(|original| original.stopped.load(Ordering::Acquire)));
    }
}

async fn until(ready: impl Fn() -> bool) {
    timeout(Duration::from_secs(2), async {
        while !ready() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
fn spawn(
    source: Option<watch::Receiver<Publication>>,
    stop: watch::Receiver<bool>,
    native: Arc<Native>,
) -> tokio::task::JoinHandle<Result<()>> {
    let retirement = native.clone();
    tokio::spawn(run(
        source,
        stop,
        move |stop| native.start(stop),
        move || retirement.settle(),
    ))
}

#[tokio::test]
async fn t1080_lock_display_and_spurious_manual_notice_preserve_original() {
    let mut source = Source::new(true);
    let native = Native::new(true);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    source.emit(Power::Running, true, false);
    stop.send_replace(false);
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert_eq!(native.count(), 1);
    assert!(!native.original(0).stopped.load(Ordering::Acquire));
    stop.send_replace(true);
    owner.await.unwrap().unwrap();
    native.conserved();
}

#[tokio::test]
async fn t1080_coalesced_stop_fences_resume_until_original_native_settles() {
    let mut source = Source::new(true);
    let native = Native::new(false);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    source.emit(Power::Suspending, false, true);
    source.emit(Power::Running, false, true);
    until(|| native.original(0).stopped.load(Ordering::Acquire)).await;
    assert_eq!(native.count(), 1);
    native.original(0).release.send_replace(true);
    until(|| native.count() == 2).await;
    assert!(native.original(0).settled.load(Ordering::Acquire));
    // A late duplicate original fence cannot settle the current attempt.
    native.original(0).release.send_replace(true);
    assert!(!native.original(1).settled.load(Ordering::Acquire));
    stop.send_replace(true);
    until(|| native.original(1).stopped.load(Ordering::Acquire)).await;
    assert!(!owner.is_finished());
    native.release_all();
    owner.await.unwrap().unwrap();
    native.conserved();
}

#[tokio::test]
async fn t1080_owner_stop_during_held_retirement_denies_resume() {
    let mut source = Source::new(true);
    let native = Native::new(false);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    source.emit(Power::Unavailable, false, true);
    until(|| native.original(0).stopped.load(Ordering::Acquire)).await;
    source.emit(Power::Running, false, true);
    drop(stop);
    native.release_all();
    owner.await.unwrap().unwrap();
    assert_eq!(native.count(), 1);
    native.conserved();
}

#[tokio::test]
async fn t1080_known_source_loss_stops_original_without_legacy_fallback() {
    let source = Source::new(true);
    let native = Native::new(true);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    drop(source);
    until(|| native.original(0).settled.load(Ordering::Acquire)).await;
    assert_eq!(native.count(), 1);
    assert!(!owner.is_finished());
    stop.send_replace(true);
    owner.await.unwrap().unwrap();
    native.conserved();
}

async fn pending_cannot_admit(publication: Publication) {
    let (source, receiver) = watch::channel(publication);
    let native = Native::new(true);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(receiver), stopped, native.clone());
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert_eq!(native.count(), 0);
    stop.send_replace(true);
    owner.await.unwrap().unwrap();
    drop(source);
}
fn corrupt_snapshot(snapshot: &mut ActivitySnapshot, index: usize) {
    if index < 4 {
        corrupt_identity(snapshot, index);
    } else {
        corrupt_demand(snapshot, index - 4);
    }
}
fn corrupt_identity(snapshot: &mut ActivitySnapshot, index: usize) {
    match index {
        0 => snapshot.demand.stamp.epoch = 0,
        1 => snapshot.auxiliary_stop.epoch = u64::MAX,
        2 => snapshot.auxiliary_stop.revision = u64::MAX,
        _ => snapshot.display_stop.revision = u64::MAX,
    }
}
fn corrupt_demand(snapshot: &mut ActivitySnapshot, index: usize) {
    match index {
        0 => snapshot.demand.auxiliary = false,
        1 => snapshot.demand.display = false,
        2 => snapshot.demand.reason = Some(Reason::PowerUnknown),
        _ => snapshot.source_lost = true,
    }
}

#[tokio::test]
async fn t1080_pending_failed_stopped_and_invalid_publications_never_admit() {
    for publication in [
        Publication::Pending,
        Publication::Failed,
        Publication::Stopped,
    ] {
        pending_cannot_admit(publication).await;
    }
    let source = Source::new(true);
    for index in 0..8 {
        let mut invalid = *source.owner.borrow();
        if let Publication::Observed { snapshot, .. } = &mut invalid {
            corrupt_snapshot(snapshot, index);
        }
        pending_cannot_admit(invalid).await;
    }
}

#[tokio::test]
async fn t1080_absent_and_explicitly_unsupported_power_keep_legacy_work() {
    for source in [None, Some(Source::new(false))] {
        let native = Native::new(true);
        let (stop, stopped) = watch::channel(false);
        let owner = spawn(
            source.as_ref().map(|s| s.owner.subscribe()),
            stopped,
            native.clone(),
        );
        until(|| native.count() == 1).await;
        drop(source);
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        assert!(!native.original(0).stopped.load(Ordering::Acquire));
        stop.send_replace(true);
        owner.await.unwrap().unwrap();
        native.conserved();
    }
}

#[tokio::test]
async fn t1080_natural_completion_failure_and_panic_each_wait_native_once() {
    for mode in 0..3 {
        let (stop, stopped) = watch::channel(false);
        let (release, released) = watch::channel(false);
        let settled = Arc::new(AtomicBool::new(false));
        let proof = settled.clone();
        let owner = tokio::spawn(run(
            None,
            stopped,
            move |_| async move {
                match mode {
                    0 => Ok(()),
                    1 => anyhow::bail!("T1080 original failure"),
                    _ => panic!("T1080 polling panic"),
                }
            },
            move || {
                let mut released = released.clone();
                let proof = proof.clone();
                async move {
                    released.wait_for(|value| *value).await.unwrap();
                    proof.store(true, Ordering::Release);
                }
            },
        ));
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        assert!(!owner.is_finished());
        release.send_replace(true);
        assert_eq!(owner.await.unwrap().is_ok(), mode == 0);
        assert!(settled.load(Ordering::Acquire));
        drop(stop);
    }
}

#[tokio::test]
async fn t1080_stop_before_admission_never_calls_native() {
    let native = Native::new(true);
    let (_stop, stopped) = watch::channel(true);
    spawn(None, stopped, native.clone()).await.unwrap().unwrap();
    assert_eq!(native.count(), 0);
}

async fn expect_transition(native: &Native, active: &mut bool, count: &mut usize, running: bool) {
    if *active == running {
        return;
    }
    if running {
        *count += 1;
        until(|| native.count() == *count).await;
    } else {
        until(|| native.original(*count - 1).settled.load(Ordering::Acquire)).await;
    }
    *active = running;
}

async fn history(mask: u8) {
    let mut source = Source::new(true);
    let native = Native::new(true);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    let mut count = 1;
    let mut active = true;
    for step in 0..6 {
        let running = mask & (1 << step) == 0;
        source.emit(
            if running {
                Power::Running
            } else {
                Power::Suspending
            },
            false,
            true,
        );
        expect_transition(&native, &mut active, &mut count, running).await;
        assert_eq!(native.count(), count);
    }
    stop.send_replace(true);
    owner.await.unwrap().unwrap();
    native.conserved();
}
#[tokio::test]
async fn t1080_bounded_64_six_transition_histories_preserve_conservation() {
    for mask in 0..64 {
        history(mask).await;
    }
}

#[tokio::test]
async fn t1080_factory_panic_still_waits_independent_native_settlement() {
    let (_stop, stopped) = watch::channel(false);
    let (release, released) = watch::channel(false);
    let factory = |_: watch::Receiver<bool>| -> std::future::Ready<Result<()>> {
        panic!("T1080 synchronous creation panic")
    };
    let owner = tokio::spawn(run(None, stopped, factory, move || {
        let mut released = released.clone();
        async move {
            released.wait_for(|value| *value).await.unwrap();
        }
    }));
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert!(!owner.is_finished());
    release.send_replace(true);
    assert!(owner
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("creation panic"));
}

#[tokio::test]
async fn t1080_late_creation_observes_original_stop_before_native_admission() {
    let (stop, stopped) = watch::channel(false);
    let (release, released) = watch::channel(false);
    let entered = Arc::new(AtomicBool::new(false));
    let attempted = entered.clone();
    let admitted = Arc::new(AtomicBool::new(false));
    let actual = admitted.clone();
    let owner = tokio::spawn(run(
        None,
        stopped,
        move |stop| {
            let mut released = released.clone();
            let attempted = attempted.clone();
            let actual = actual.clone();
            async move {
                attempted.store(true, Ordering::Release);
                released.wait_for(|value| *value).await.unwrap();
                if !*stop.borrow() {
                    actual.store(true, Ordering::Release);
                }
                Ok(())
            }
        },
        || async {},
    ));
    until(|| entered.load(Ordering::Acquire)).await;
    stop.send_replace(true);
    for _ in 0..8 {
        tokio::task::yield_now().await;
    }
    assert!(!owner.is_finished());
    release.send_replace(true);
    owner.await.unwrap().unwrap();
    assert!(!admitted.load(Ordering::Acquire));
}

#[tokio::test]
async fn t1080_failed_original_cleanup_never_automatically_replays_current_intent() {
    let mut source = Source::new(true);
    let native = Native::new(false);
    let starting = native.clone();
    let retirement = native.clone();
    let (_stop, stopped) = watch::channel(false);
    let owner = tokio::spawn(run(
        Some(source.owner.subscribe()),
        stopped,
        move |stop| {
            let work = starting.start(stop);
            async move {
                work.await?;
                anyhow::bail!("T1080 original cleanup failed")
            }
        },
        move || retirement.settle(),
    ));
    until(|| native.count() == 1).await;
    source.emit(Power::Suspending, false, true);
    until(|| native.original(0).stopped.load(Ordering::Acquire)).await;
    source.emit(Power::Running, false, true);
    assert!(!owner.is_finished());
    native.release_all();
    assert!(owner.await.unwrap().is_err());
    assert_eq!(native.count(), 1);
    native.conserved();
}

#[tokio::test]
async fn t1080_valid_initial_zero_revision_is_admissible() {
    let source = Source::new(true);
    let mut initial = *source.owner.borrow();
    if let Publication::Observed { snapshot, .. } = &mut initial {
        snapshot.demand.stamp.revision = 0;
        snapshot.validate().unwrap();
    }
    source.owner.send_replace(initial);
    let native = Native::new(true);
    let (stop, stopped) = watch::channel(false);
    let owner = spawn(Some(source.owner.subscribe()), stopped, native.clone());
    until(|| native.count() == 1).await;
    stop.send_replace(true);
    owner.await.unwrap().unwrap();
    native.conserved();
}

fn retained_blocking_original(
    mut stop: watch::Receiver<bool>,
    entered: Arc<AtomicBool>,
    cutoff: Arc<AtomicBool>,
    release: std::sync::mpsc::Receiver<()>,
) -> Result<()> {
    entered.store(true, Ordering::Release);
    while stop.has_changed().is_ok() {
        if *stop.borrow_and_update() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    cutoff.store(true, Ordering::Release);
    let _ = release.recv(); // Sender loss also settles this owned memory-only fixture.
    Ok(())
}

#[tokio::test]
async fn t1080_outer_cancellation_keeps_original_native_budget_until_actual_join() {
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let (release, gate) = std::sync::mpsc::channel();
    let entered = Arc::new(AtomicBool::new(false));
    let active = entered.clone();
    let cutoff = Arc::new(AtomicBool::new(false));
    let observed = cutoff.clone();
    let (_manual, manual) = watch::channel(false);
    let originals = Arc::new(Mutex::new(Some((admission, gate))));
    let completion = Arc::new(Mutex::new(None));
    let published = completion.clone();
    let owner = tokio::spawn(run(
        None,
        manual,
        move |stop| {
            let (admission, gate) = originals.lock().unwrap().take().unwrap();
            let active = active.clone();
            let observed = observed.clone();
            let mut original = crate::owned_task::Task::spawn_blocking(move || {
                retained_blocking_original(stop, active, observed, gate)
            })
            .unwrap();
            original.keep_alive(admission);
            *published.lock().unwrap() = Some(original.completion());
            async move {
                original.await??;
                Ok(())
            }
        },
        || async {},
    ));
    until(|| entered.load(Ordering::Acquire)).await;
    owner.abort();
    assert!(owner.await.unwrap_err().is_cancelled());
    until(|| cutoff.load(Ordering::Acquire)).await;
    let original = completion.lock().unwrap().clone().unwrap();
    assert!(!original.is_complete());
    assert_eq!(pool.active(), 1);
    assert!(pool.reserve().is_err());
    release.send(()).unwrap();
    until(|| original.is_complete()).await;
    assert_eq!(pool.active(), 0);
}

struct DropBoundary(Arc<AtomicBool>);
impl Future for DropBoundary {
    type Output = Result<()>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::task::Poll::Ready(Ok(()))
    }
}
impl Drop for DropBoundary {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
#[tokio::test]
async fn t1080_completed_attempt_destroys_its_future_before_native_settlement() {
    let (_stop, stopped) = watch::channel(false);
    let destroyed = Arc::new(AtomicBool::new(false));
    let original = destroyed.clone();
    let proof = destroyed.clone();
    run(
        None,
        stopped,
        move |_| DropBoundary(original.clone()),
        move || {
            let proof = proof.clone();
            async move {
                assert!(proof.load(Ordering::Acquire));
            }
        },
    )
    .await
    .unwrap();
    assert!(destroyed.load(Ordering::Acquire));
}

#[path = "progress_tests.rs"]
mod progress_tests;
