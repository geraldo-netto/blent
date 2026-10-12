//! T1008: read-only Linux activity observations, independently owned per backend.
use crate::owned_task::Task;
use anyhow::{ensure, Result};
use blent_config::host_activity::{Capabilities, Event, HostState, Observations, Source};
use futures_util::FutureExt;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::Notify;
mod native;

pub use blent_config::host_activity::SourceSettings as Settings;
fn compositor_desktop(session: &str, desktop: &str) -> String {
    if session == "wayland" {
        desktop.to_owned()
    } else {
        String::new()
    }
}
fn epoch(counter: &AtomicU64) -> Result<u64> {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .map(|value| value + 1)
        .map_err(|_| anyhow::anyhow!("activity source epochs exhausted"))
}
struct Shared {
    observations: Mutex<Observations>,
    stopped: AtomicBool,
    wake: Notify,
}
impl Shared {
    fn state(&self) -> std::sync::MutexGuard<'_, Observations> {
        self.observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn update(&self, patch: native::Patch) -> Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut observations = self.state();
        let state = patch.apply(observations.current());
        observations.publish(state)?;
        Ok(())
    }
    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.state().stop();
        self.wake.notify_waiters();
    }
}
/// Each source has an immutable capability set and fresh epoch. A disappeared
/// service becomes unknown on its own fields; a replacement cannot inherit it.
pub struct LinuxSource {
    epoch: u64,
    capabilities: Capabilities,
    shared: Arc<Shared>,
    tasks: Mutex<Vec<Task>>,
}
impl LinuxSource {
    pub async fn connect(settings: Settings) -> Result<Self> {
        settings.validate()?;
        let system = tokio::time::timeout(settings.deadline, zbus::Connection::system())
            .await
            .ok()
            .and_then(Result::ok);
        let session = tokio::time::timeout(settings.deadline, zbus::Connection::session())
            .await
            .ok()
            .and_then(Result::ok);
        // Compositor properties establish compositor-managed state on Wayland.
        // Direct X11 DPMS can bypass them; its independent source is T1013.
        let desktop = compositor_desktop(
            &std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
            &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        );
        Self::on_buses(
            system,
            session,
            settings,
            std::env::var("XDG_SESSION_ID").ok(),
            &desktop,
        )
        .await
    }
    async fn on_buses(
        system: Option<zbus::Connection>,
        session: Option<zbus::Connection>,
        settings: Settings,
        session_id: Option<String>,
        desktop: &str,
    ) -> Result<Self> {
        settings.validate()?;
        static EPOCH: AtomicU64 = AtomicU64::new(0);
        let epoch = epoch(&EPOCH)?;
        let mut actors = Vec::new();
        if let Some(bus) = system {
            match tokio::time::timeout(
                settings.deadline,
                native::Actor::logind(bus, session_id, settings.queue, settings.deadline),
            )
            .await
            {
                Ok(Ok(actor)) => actors.push(actor),
                error => tracing::warn!(?error, "Linux power/lock observations unsupported"),
            }
        }
        if let (Some(bus), Some(target)) = (session, native::Target::desktop(desktop)) {
            match tokio::time::timeout(
                settings.deadline,
                native::Actor::display(bus, target, settings.queue, settings.deadline),
            )
            .await
            {
                Ok(Ok(actor)) => actors.push(actor),
                error => tracing::warn!(
                    ?error,
                    "Linux compositor display-power observations unsupported"
                ),
            }
        }
        Self::start(epoch, settings, actors)
    }
    fn start(epoch: u64, settings: Settings, actors: Vec<native::Actor>) -> Result<Self> {
        let mut capabilities = Capabilities::default();
        let mut state = HostState::default();
        for actor in &actors {
            actor.include(&mut capabilities, &mut state);
        }
        let mut observations = Observations::new(epoch, capabilities, settings.queue)?;
        observations.publish(state)?;
        let shared = Arc::new(Shared {
            observations: Mutex::new(observations),
            stopped: AtomicBool::new(false),
            wake: Notify::new(),
        });
        let mut source = Self {
            epoch,
            capabilities,
            shared,
            tasks: Mutex::new(Vec::new()),
        };
        for actor in actors {
            if let Err(error) = source.spawn(actor) {
                source.stop();
                return Err(error);
            }
        }
        Ok(source)
    }
    fn spawn(&self, actor: native::Actor) -> Result<()> {
        let shared = self.shared.clone();
        let task = Task::spawn(async move { actor.run(shared).await })?;
        self.tasks.lock().unwrap().push(task);
        Ok(())
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
impl Source for LinuxSource {
    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    fn poll(&mut self) -> Result<Option<Event>> {
        self.shared.state().poll()
    }
    fn stop(&mut self) {
        self.shared.stop();
    }
    fn retired(&self) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain_mut(|task| task.now_or_never().is_none());
        tasks.is_empty()
    }
}
impl Drop for LinuxSource {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(test)]
mod tests;
