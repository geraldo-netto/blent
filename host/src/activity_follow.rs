//! Portable orchestration; creation/native adapters retain the independent lease.
use crate::owned_task::Task;
use anyhow::{ensure, Result};
use blent_config::{
    host_activity::*,
    retirement::{Completion, Pool},
};
use futures_util::FutureExt;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
use tokio::sync::{watch, Notify};

#[derive(Clone, Copy)]
pub struct Settings {
    pub source: SourceSettings,
    pub startup_deadline: Duration,
    pub interval: Duration,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            source: SourceSettings::default(),
            startup_deadline: Duration::from_secs(10),
            interval: Duration::from_millis(50),
        }
    }
}
impl Settings {
    pub fn validate(self) -> Result<()> {
        self.source.validate()?;
        ensure!(
            !self.startup_deadline.is_zero() && self.startup_deadline <= Duration::from_secs(600),
            "invalid activity creation deadline"
        );
        ensure!(
            (Duration::from_millis(1)..=Duration::from_secs(1)).contains(&self.interval),
            "invalid activity observation interval"
        );
        Ok(())
    }
}
struct Shared {
    updates: watch::Sender<Publication>,
    stopped: AtomicBool,
    publication: Mutex<()>,
    wake: Notify,
}
impl Shared {
    fn publish(&self, value: Publication) {
        let _lock = self
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.stopped.load(Ordering::Acquire) {
            self.updates.send_if_modified(|current| {
                if *current == value {
                    return false;
                }
                *current = value;
                true
            });
        }
    }
    fn stop(&self) {
        let _lock = self
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.stopped.store(true, Ordering::Release);
        self.updates.send_replace(Publication::Stopped);
        self.wake.notify_waiters();
    }
    async fn stopped(&self) {
        let wake = self.wake.notified();
        tokio::pin!(wake);
        wake.as_mut().enable();
        if !self.stopped.load(Ordering::Acquire) {
            wake.await;
        }
    }
}
pub struct Following {
    shared: Arc<Shared>,
    task: Mutex<Option<Task>>,
    native: Completion,
}
impl Following {
    pub fn start(
        factory: Arc<dyn SourceFactory>,
        settings: Settings,
        preferences: blent_config::host_activity::Settings,
    ) -> Result<Self> {
        settings.validate()?;
        let pool = pool()?;
        let owner = SourceLifetime::new(pool.reserve()?);
        let native = owner.completion();
        let (updates, _) = watch::channel(Publication::Pending);
        let shared = Arc::new(Shared {
            updates,
            stopped: AtomicBool::new(false),
            publication: Mutex::new(()),
            wake: Notify::new(),
        });
        let context = shared.clone();
        let lifetime = owner.clone();
        let mut task =
            Task::spawn(
                async move { run(factory, settings, preferences, lifetime, context).await },
            )?;
        owner.retain(task.completion());
        task.keep_alive(owner);
        Ok(Self {
            shared,
            task: Mutex::new(Some(task)),
            native,
        })
    }
    pub fn subscribe(&self) -> watch::Receiver<Publication> {
        self.shared.updates.subscribe()
    }
    pub fn stop(&self) {
        self.shared.stop();
    }
    pub fn retired(&self) -> bool {
        let mut task = self
            .task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(work) = task.as_mut() {
            if work.now_or_never().is_none() {
                return false;
            }
            *task = None;
        }
        self.native.is_complete()
    }
}
impl Drop for Following {
    fn drop(&mut self) {
        self.stop();
    }
}
fn pool() -> Result<&'static Pool> {
    static POOL: OnceLock<std::io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(blent_config::retirement_limits::configured(
            "BLENT_ACTIVITY_LIMIT",
            1,
            4,
        )?)
    })
    .as_ref()
    .map_err(|error| anyhow::anyhow!("activity admission: {error}"))
}
async fn run(
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    preferences: blent_config::host_activity::Settings,
    lifetime: SourceLifetime,
    shared: Arc<Shared>,
) -> Result<()> {
    let creation = std::panic::AssertUnwindSafe(async {
        tokio::select! { biased;
            _ = shared.stopped() => anyhow::bail!("activity creation stopped"),
            value = tokio::time::timeout(settings.startup_deadline, factory.create(settings.source, lifetime)) => Ok(value??),
        }
    }).catch_unwind().await;
    let mut binding =
        match creation.unwrap_or_else(|_| Err(anyhow::anyhow!("activity creator panicked"))) {
            Ok(binding) => binding,
            Err(error) => {
                tracing::warn!(%error, "activity source creation failed");
                shared.publish(Publication::Failed);
                return Ok(());
            }
        };
    let outcome =
        std::panic::AssertUnwindSafe(observe(&mut binding, settings, preferences, &shared))
            .catch_unwind()
            .await;
    if !matches!(outcome, Ok(Ok(()))) {
        tracing::warn!(?outcome, "activity source following failed");
        shared.publish(Publication::Failed);
    }
    binding.source.stop();
    while !binding.source.retired() {
        tokio::time::sleep(settings.interval).await;
    }
    Ok(())
}
async fn observe(
    binding: &mut StartedSource,
    settings: Settings,
    preferences: blent_config::host_activity::Settings,
    shared: &Shared,
) -> Result<()> {
    let capabilities = binding.source.capabilities();
    let mut feed = Feed::new(binding.epoch, capabilities, preferences)?;
    let mut timer = tokio::time::interval(settings.interval);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if shared.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        if let Err(error) = drain(binding.source.as_mut(), &mut feed, settings.source.queue) {
            feed.lost();
            shared.publish(Publication::Observed {
                capabilities,
                snapshot: feed.snapshot(true, false),
            });
            tracing::warn!(%error, "original activity observation lost");
            return Ok(());
        }
        shared.publish(Publication::Observed {
            capabilities,
            snapshot: feed.snapshot(true, false),
        });
        tokio::select! { biased;
            _ = shared.stopped() => return Ok(()),
            _ = timer.tick() => {},
        }
    }
}
fn drain(source: &mut dyn Source, feed: &mut Feed, capacity: usize) -> Result<()> {
    for _ in 0..capacity {
        let Some(event) = source.poll()? else {
            return Ok(());
        };
        feed.observe(event)?;
    }
    // A producer may refill while we consume. Capacity bounds one turn's work;
    // the source owns enforcement of its actual outstanding queue extent.
    Ok(())
}
#[cfg(test)]
#[path = "activity_follow_tests.rs"]
mod tests;
