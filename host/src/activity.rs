//! T1008: read-only Linux activity observations, independently owned per backend.
use crate::owned_task::Task;
use anyhow::{ensure, Result};
use blent_config::host_activity::{
    Capabilities, Event, HostState, Observations, Source, SourceLifetime,
};
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
mod power;
pub use power::PowerFactory;

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
fn fresh_epoch() -> Result<u64> {
    static EPOCH: AtomicU64 = AtomicU64::new(0);
    epoch(&EPOCH)
}
/// One backend owns its native proof independently of its task result.
trait Backend: Send {
    fn include(&self, caps: &mut Capabilities, state: &mut HostState);
    fn spawn(self: Box<Self>, shared: Arc<Shared>) -> Result<Work>;
}
struct Work {
    task: Option<Task>,
    native: Arc<Mutex<Vec<blent_config::retirement::Completion>>>,
}
impl Work {
    fn asynchronous(task: Task) -> Self {
        Self {
            task: Some(task),
            native: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn retired(&mut self) -> bool {
        if let Some(task) = &mut self.task {
            if task.now_or_never().is_none() {
                return false;
            }
            self.task = None;
        }
        self.native
            .lock()
            .unwrap()
            .iter()
            .all(blent_config::retirement::Completion::is_complete)
    }
}
impl Backend for native::Actor {
    fn include(&self, caps: &mut Capabilities, state: &mut HostState) {
        self.include(caps, state);
    }
    fn spawn(self: Box<Self>, shared: Arc<Shared>) -> Result<Work> {
        Ok(Work::asynchronous(Task::spawn(async move {
            self.run(shared).await
        })?))
    }
}
mod dpms;
struct Shared {
    observations: Mutex<Observations>,
    stopped: AtomicBool,
    wake: Notify,
    lifetime: SourceLifetime,
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
    tasks: Mutex<Vec<Work>>,
}
impl LinuxSource {
    pub async fn connect(settings: Settings, lifetime: SourceLifetime) -> Result<Self> {
        settings.validate()?;
        let epoch = fresh_epoch()?; // Reserve identity before acquiring any native owner.
        let system = power::system_bus(settings).await;
        let session = tokio::time::timeout(settings.deadline, zbus::Connection::session())
            .await
            .ok()
            .and_then(Result::ok);
        let session_kind = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let desktop = compositor_desktop(
            &session_kind,
            &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        );
        let mut actors = Self::discover(
            system,
            session,
            settings,
            std::env::var("XDG_SESSION_ID").ok(),
            &desktop,
        )
        .await;
        if session_kind == "x11" {
            match dpms::Probe::start(
                dpms::Program::current(settings)?,
                settings,
                lifetime.clone(),
            )
            .await
            {
                Ok(probe) => actors.push(Box::new(probe)),
                Err(error) => tracing::warn!(%error, "X11 display-power observation unavailable"),
            }
        }
        Self::start(epoch, settings, actors, lifetime)
    }
    async fn discover(
        system: Option<zbus::Connection>,
        session: Option<zbus::Connection>,
        settings: Settings,
        session_id: Option<String>,
        desktop: &str,
    ) -> Vec<Box<dyn Backend>> {
        let mut actors: Vec<Box<dyn Backend>> = Vec::new();
        if let Some(bus) = system {
            match tokio::time::timeout(
                settings.deadline,
                native::Actor::logind(bus, session_id, settings.queue, settings.deadline),
            )
            .await
            {
                Ok(Ok(actor)) => actors.push(Box::new(actor)),
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
                Ok(Ok(actor)) => actors.push(Box::new(actor)),
                error => tracing::warn!(
                    ?error,
                    "Linux compositor display-power observations unsupported"
                ),
            }
        }
        actors
    }
    fn start(
        epoch: u64,
        settings: Settings,
        actors: Vec<Box<dyn Backend>>,
        lifetime: SourceLifetime,
    ) -> Result<Self> {
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
            lifetime,
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
    fn spawn(&self, actor: Box<dyn Backend>) -> Result<()> {
        let mut work = actor.spawn(self.shared.clone())?;
        self.shared.lifetime.retain(
            work.task
                .as_ref()
                .expect("unexposed activity task")
                .completion(),
        );
        work.task
            .as_mut()
            .expect("unexposed activity task")
            .keep_alive(self.shared.lifetime.clone());
        self.tasks.lock().unwrap().push(work);
        Ok(())
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
/// Native implementation of the independently defined source factory contract.
pub struct Factory;
impl blent_config::host_activity::SourceFactory for Factory {
    fn create(
        &self,
        settings: Settings,
        lifetime: SourceLifetime,
    ) -> blent_config::host_activity::SourceCreation {
        Box::pin(async move {
            let source = LinuxSource::connect(settings, lifetime).await?;
            Ok(blent_config::host_activity::StartedSource {
                epoch: source.epoch(),
                source: Box::new(source),
            })
        })
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
        tasks.retain_mut(|work| !work.retired());
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
#[cfg(test)]
pub(crate) use tests::private as private_bus_test;
