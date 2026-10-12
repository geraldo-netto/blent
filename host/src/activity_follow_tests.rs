//! T1018: source ownership precedes asynchronous constructor exposure.
use crate::activity_follow::{Following, Settings};
use blent_config::host_activity::{Publication, SourceCreation, SourceFactory, SourceLifetime};
struct Stalled;
impl SourceFactory for Stalled {
    fn create(
        &self,
        _: blent_config::host_activity::SourceSettings,
        _: SourceLifetime,
    ) -> SourceCreation {
        Box::pin(std::future::pending())
    }
}
#[tokio::test]
async fn t1018_pending_creation_stop_is_prompt_and_observed_separately_from_retirement() {
    let following = Following::start(
        std::sync::Arc::new(Stalled),
        Settings::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(*following.subscribe().borrow(), Publication::Pending);
    following.stop();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !following.retired() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(*following.subscribe().borrow(), Publication::Stopped);
}
struct Replenished {
    revision: u64,
}
impl blent_config::host_activity::Source for Replenished {
    fn capabilities(&self) -> blent_config::host_activity::Capabilities {
        Default::default()
    }
    fn poll(&mut self) -> anyhow::Result<Option<blent_config::host_activity::Event>> {
        self.revision += 1;
        Ok(Some(blent_config::host_activity::Event {
            epoch: 7,
            revision: self.revision,
            state: Default::default(),
        }))
    }
    fn stop(&mut self) {}
    fn retired(&self) -> bool {
        true
    }
}
#[test]
fn t1018_bounded_drain_accepts_a_source_replenished_concurrently_with_consumption() {
    let mut source = Replenished { revision: 0 };
    let mut feed =
        blent_config::host_activity::Feed::new(7, Default::default(), Default::default()).unwrap();
    super::drain(&mut source, &mut feed, 1).unwrap();
    assert_eq!(source.revision, 1);
}

use blent_config::host_activity::{Capabilities, Event, HostState, Power, Source, StartedSource};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
struct State {
    events: Mutex<VecDeque<Event>>,
    stop: AtomicBool,
    released: AtomicBool,
    failure: std::sync::atomic::AtomicU8,
}
struct Ready {
    state: Arc<State>,
    caps: Capabilities,
    epoch: u64,
}
struct Input {
    state: Arc<State>,
    caps: Capabilities,
}
impl Source for Input {
    fn capabilities(&self) -> Capabilities {
        self.caps
    }
    fn poll(&mut self) -> anyhow::Result<Option<Event>> {
        match self.state.failure.load(Ordering::Acquire) {
            1 => anyhow::bail!("T1018 input lost"),
            2 => panic!("T1018 input panic"),
            _ => Ok(self.state.events.lock().unwrap().pop_front()),
        }
    }
    fn stop(&mut self) {
        self.state.stop.store(true, Ordering::Release);
    }
    fn retired(&self) -> bool {
        self.state.released.load(Ordering::Acquire)
    }
}
impl SourceFactory for Ready {
    fn create(
        &self,
        _: blent_config::host_activity::SourceSettings,
        _: SourceLifetime,
    ) -> SourceCreation {
        let source = Input {
            state: self.state.clone(),
            caps: self.caps,
        };
        let epoch = self.epoch;
        Box::pin(async move {
            Ok(StartedSource {
                epoch,
                source: Box::new(source),
            })
        })
    }
}
fn state() -> Arc<State> {
    Arc::new(State {
        events: Mutex::new(VecDeque::new()),
        stop: AtomicBool::new(false),
        released: AtomicBool::new(false),
        failure: std::sync::atomic::AtomicU8::new(0),
    })
}
fn running(revision: u64) -> Event {
    Event {
        epoch: 7,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(false),
            display_on: Some(true),
        },
    }
}
fn caps() -> Capabilities {
    Capabilities {
        power: true,
        lock: true,
        display: true,
    }
}
fn settings() -> Settings {
    Settings {
        interval: Duration::from_millis(1),
        ..Settings::default()
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn finish(following: &Following, state: &State) {
    following.stop();
    assert!(!following.retired());
    state.released.store(true, Ordering::Release);
    until(|| following.retired()).await;
}
#[tokio::test]
async fn t1018_ordered_edges_survive_coalescing_and_stop_waits_for_actual_source_retirement() {
    let state = state();
    state.events.lock().unwrap().extend([
        running(1),
        Event {
            state: HostState {
                locked: Some(true),
                ..running(2).state
            },
            ..running(2)
        },
        running(3),
    ]);
    let following = Following::start(
        Arc::new(Ready {
            state: state.clone(),
            caps: caps(),
            epoch: 7,
        }),
        settings(),
        Default::default(),
    )
    .unwrap();
    let reader = following.subscribe();
    until(|| matches!(*reader.borrow(), Publication::Observed { snapshot, .. } if snapshot.demand.stamp.revision == 3)).await;
    let Publication::Observed { snapshot, .. } = *reader.borrow() else {
        unreachable!()
    };
    assert!(snapshot.demand.display && snapshot.demand.auxiliary);
    assert_eq!(snapshot.display_stop.revision, 2);
    assert_eq!(snapshot.auxiliary_stop.revision, 0);
    following.stop();
    until(|| state.stop.load(Ordering::Acquire)).await;
    assert!(!following.retired());
    state.events.lock().unwrap().push_back(running(4));
    assert_eq!(*reader.borrow(), Publication::Stopped);
    state.released.store(true, Ordering::Release);
    until(|| following.retired()).await;
    assert_eq!(*reader.borrow(), Publication::Stopped);
}
#[tokio::test]
async fn t1018_source_loss_and_panics_fail_without_freeing_stuck_native_ownership() {
    for failure in [1, 2] {
        let state = state();
        state.events.lock().unwrap().push_back(running(1));
        let following = Following::start(
            Arc::new(Ready {
                state: state.clone(),
                caps: caps(),
                epoch: 7,
            }),
            settings(),
            Default::default(),
        )
        .unwrap();
        let reader = following.subscribe();
        until(|| matches!(*reader.borrow(), Publication::Observed { .. })).await;
        state.failure.store(failure, Ordering::Release);
        until(|| state.stop.load(Ordering::Acquire)).await;
        assert!(!following.retired());
        assert!(Following::start(Arc::new(Stalled), settings(), Default::default()).is_err());
        if failure == 1 {
            assert!(
                matches!(*reader.borrow(), Publication::Observed { snapshot, .. } if snapshot.source_lost && !snapshot.demand.auxiliary)
            );
        } else {
            assert_eq!(*reader.borrow(), Publication::Failed);
        }
        finish(&following, &state).await;
    }
}
struct Failed {
    panic: bool,
}
impl SourceFactory for Failed {
    fn create(
        &self,
        _: blent_config::host_activity::SourceSettings,
        _: SourceLifetime,
    ) -> SourceCreation {
        let panic = self.panic;
        Box::pin(async move {
            if panic {
                panic!("T1018 creator panic");
            }
            anyhow::bail!("T1018 creator error")
        })
    }
}
#[tokio::test]
async fn t1018_failed_timed_out_and_invalid_creation_never_publish_a_grant() {
    for factory in [
        Arc::new(Stalled) as Arc<dyn SourceFactory>,
        Arc::new(Failed { panic: false }),
        Arc::new(Failed { panic: true }),
    ] {
        let following = Following::start(
            factory,
            Settings {
                startup_deadline: Duration::from_millis(10),
                ..settings()
            },
            Default::default(),
        )
        .unwrap();
        let reader = following.subscribe();
        until(|| *reader.borrow() == Publication::Failed).await;
        until(|| following.retired()).await;
    }
    let state = state();
    let following = Following::start(
        Arc::new(Ready {
            state: state.clone(),
            caps: caps(),
            epoch: 0,
        }),
        settings(),
        Default::default(),
    )
    .unwrap();
    until(|| state.stop.load(Ordering::Acquire)).await;
    assert_eq!(*following.subscribe().borrow(), Publication::Failed);
    finish(&following, &state).await;
}
struct Held {
    entered: Arc<tokio::sync::Notify>,
    release: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}
impl SourceFactory for Held {
    fn create(
        &self,
        _: blent_config::host_activity::SourceSettings,
        lifetime: SourceLifetime,
    ) -> SourceCreation {
        let entered = self.entered.clone();
        let release = self.release.lock().unwrap().take().unwrap();
        let native = lifetime.clone();
        let mut task = crate::owned_task::Task::spawn_blocking(move || {
            entered.notify_one();
            release.recv().unwrap();
            drop(native);
            Ok(())
        })
        .unwrap();
        task.keep_alive(lifetime);
        Box::pin(async move {
            let _task = task;
            std::future::pending().await
        })
    }
}
#[tokio::test]
async fn t1018_cancelled_or_dropped_creator_keeps_capacity_until_actual_native_join() {
    for drop_owner in [false, true] {
        let entered = Arc::new(tokio::sync::Notify::new());
        let (release, wait) = std::sync::mpsc::channel();
        let following = Following::start(
            Arc::new(Held {
                entered: entered.clone(),
                release: Mutex::new(Some(wait)),
            }),
            settings(),
            Default::default(),
        )
        .unwrap();
        let proof = following.native.clone();
        tokio::time::timeout(Duration::from_secs(3), entered.notified())
            .await
            .unwrap();
        following.stop();
        if drop_owner {
            drop(following);
        } else {
            until(|| {
                assert!(!following.retired());
                following.task.lock().unwrap().is_none()
            })
            .await;
            drop(following);
        }
        assert!(!proof.is_complete());
        assert!(Following::start(Arc::new(Stalled), settings(), Default::default()).is_err());
        release.send(()).unwrap();
        until(|| proof.is_complete()).await;
    }
}
#[test]
fn t1018_bounded_settings_reject_invalid_ranges_before_any_factory_exposure() {
    for millis in [0, 1, 50, 1000, 1001, u64::MAX] {
        let settings = Settings {
            interval: Duration::from_millis(millis),
            ..Settings::default()
        };
        assert_eq!(settings.validate().is_ok(), (1..=1000).contains(&millis));
    }
    for seconds in [0, 1, 10, 600, 601, u64::MAX] {
        let settings = Settings {
            startup_deadline: Duration::from_secs(seconds),
            ..Settings::default()
        };
        assert_eq!(settings.validate().is_ok(), (1..=600).contains(&seconds));
    }
    assert!(Following::start(
        Arc::new(Stalled),
        Settings {
            interval: Duration::ZERO,
            ..Settings::default()
        },
        Default::default()
    )
    .is_err());
    assert!(Settings {
        source: blent_config::host_activity::SourceSettings {
            queue: 0,
            ..Default::default()
        },
        ..Settings::default()
    }
    .validate()
    .is_err());
}

#[test]
fn t1018_identical_publications_and_late_updates_after_stop_do_not_wake_consumers() {
    let (updates, mut reader) = tokio::sync::watch::channel(Publication::Pending);
    let shared = super::Shared {
        updates,
        stopped: AtomicBool::new(false),
        publication: Mutex::new(()),
        wake: tokio::sync::Notify::new(),
    };
    shared.publish(Publication::Pending);
    assert!(!reader.has_changed().unwrap());
    shared.publish(Publication::Failed);
    assert!(reader.has_changed().unwrap());
    reader.borrow_and_update();
    shared.publish(Publication::Failed);
    assert!(!reader.has_changed().unwrap());
    shared.stop();
    assert_eq!(*reader.borrow_and_update(), Publication::Stopped);
    shared.publish(Publication::Failed);
    assert!(!reader.has_changed().unwrap());
}
