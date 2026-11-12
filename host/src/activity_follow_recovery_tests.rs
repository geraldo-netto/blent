//! T1343: replacement requires the original source and native proof to retire.
use super::tests::{caps, running, settings, state, until, Input, State};
use super::*;
use std::{collections::VecDeque, sync::atomic::AtomicUsize};

struct Attempt {
    state: Arc<State>,
    epoch: u64,
    caps: Capabilities,
    native: Option<Completion>,
}
struct Sequence {
    attempts: Mutex<VecDeque<Option<Attempt>>>,
    calls: AtomicUsize,
}
impl SourceFactory for Sequence {
    fn create(&self, _: SourceSettings, lifetime: SourceLifetime) -> SourceCreation {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let attempt = self.attempts.lock().unwrap().pop_front().flatten();
        Box::pin(async move {
            let attempt = attempt.ok_or_else(|| anyhow::anyhow!("T1343 unavailable source"))?;
            if let Some(native) = attempt.native {
                lifetime.retain(native);
            }
            Ok(StartedSource {
                epoch: attempt.epoch,
                source: Box::new(Input {
                    state: attempt.state,
                    caps: attempt.caps,
                }),
            })
        })
    }
}
fn attempt(epoch: u64, caps: Capabilities) -> Attempt {
    let state = state();
    state.events.lock().unwrap().push_back(Event {
        epoch,
        ..running(1)
    });
    Attempt {
        state,
        epoch,
        caps,
        native: None,
    }
}
fn sequence(attempts: Vec<Option<Attempt>>) -> Arc<Sequence> {
    Arc::new(Sequence {
        attempts: Mutex::new(attempts.into()),
        calls: AtomicUsize::new(0),
    })
}
fn start(factory: Arc<Sequence>) -> Following {
    Following::start(
        factory,
        Settings {
            source: SourceSettings {
                deadline: Duration::from_millis(20),
                ..Default::default()
            },
            ..settings()
        },
        Default::default(),
    )
    .unwrap()
}
async fn observed(following: &Following, epoch: u64) {
    until(|| {
        matches!(*following.subscribe().borrow(), Publication::Observed { snapshot, .. }
        if snapshot.demand.stamp.epoch == epoch && snapshot.demand.display)
    })
    .await;
}
async fn lose(state: &State) {
    state.failure.store(1, Ordering::Release);
    until(|| state.stop.load(Ordering::Acquire)).await;
}
async fn finish(following: Following, state: &State) {
    following.stop();
    state.released.store(true, Ordering::Release);
    until(|| following.retired()).await;
}
#[tokio::test]
async fn t1343_loss_replaces_only_after_original_native_fence_with_fresh_evidence() {
    let native_pool = Pool::new(1).unwrap();
    let native = native_pool.reserve().unwrap();
    let mut original = attempt(7, caps());
    original.native = Some(native.completion());
    let original_state = original.state.clone();
    let replacement = attempt(8, caps());
    let replacement_state = replacement.state.clone();
    let factory = sequence(vec![Some(original), Some(replacement)]);
    let following = start(factory.clone());
    observed(&following, 7).await;
    lose(&original_state).await;
    original_state.released.store(true, Ordering::Release);
    tokio::time::sleep(Duration::from_millis(70)).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
    assert!(!following.retired());
    assert!(
        matches!(*following.subscribe().borrow(), Publication::Observed { snapshot, .. }
        if snapshot.source_lost && !snapshot.demand.display)
    );
    drop(native);
    observed(&following, 8).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 2);
    original_state.events.lock().unwrap().push_back(running(99));
    observed(&following, 8).await;
    finish(following, &replacement_state).await;
}
#[tokio::test]
async fn t1343_failed_replacement_retries_and_stop_prevents_further_creation() {
    let original = attempt(7, caps());
    let original_state = original.state.clone();
    let replacement = attempt(9, caps());
    let replacement_state = replacement.state.clone();
    let factory = sequence(vec![Some(original), None, Some(replacement)]);
    let following = start(factory.clone());
    observed(&following, 7).await;
    lose(&original_state).await;
    original_state.released.store(true, Ordering::Release);
    observed(&following, 9).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 3);
    lose(&replacement_state).await;
    finish(following, &replacement_state).await;
    assert_eq!(factory.calls.load(Ordering::Acquire), 3);
}
#[tokio::test]
async fn t1343_replacement_rejects_old_epochs_and_uses_fresh_capabilities() {
    for epoch in [0, 6, 7, 8] {
        let original = attempt(7, caps());
        let original_state = original.state.clone();
        let replacement = attempt(epoch, Capabilities::default());
        replacement.state.events.lock().unwrap().clear();
        let replacement_state = replacement.state.clone();
        let factory = sequence(vec![Some(original), Some(replacement)]);
        let following = start(factory);
        observed(&following, 7).await;
        lose(&original_state).await;
        original_state.released.store(true, Ordering::Release);
        until(|| {
            matches!(
                *following.subscribe().borrow(),
                Publication::Failed
                    | Publication::Observed {
                        capabilities: Capabilities {
                            power: false,
                            lock: false,
                            display: false
                        },
                        ..
                    }
            )
        })
        .await;
        if epoch <= 7 {
            assert_eq!(*following.subscribe().borrow(), Publication::Failed);
        } else {
            assert_eq!(
                *following.subscribe().borrow(),
                Publication::Observed {
                    capabilities: Capabilities::default(),
                    snapshot: Feed::new(epoch, Capabilities::default(), Default::default())
                        .unwrap()
                        .snapshot(true, false),
                }
            );
        }
        finish(following, &replacement_state).await;
    }
}
