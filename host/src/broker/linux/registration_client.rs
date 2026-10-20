//! Retained Linux media registration. Logical close is prompt; capacity and
//! listener capabilities survive until both native exit and broker retirement.
use super::{
    client::{Connector, Launch},
    lifetime, Endpoints,
};
use anyhow::{ensure, Result};
use blent_config::{
    connection::registration_client::{Client, Failure, Model, Phase, Request, Status},
    connection::resources::AdmissionTimeout,
    retirement::{Completion, Observe, Pool},
    retirement_work,
};
use std::{
    net::TcpListener,
    os::fd::OwnedFd,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use tokio::{sync::Notify, time::Instant};
mod worker;

#[derive(Clone)]
pub struct Settings {
    pub launch: Launch,
    pub capacity: usize,
    pub startup: Duration,
    pub admission: Duration,
    pub cleanup: Duration,
}
impl Settings {
    pub fn new(launch: Launch) -> Self {
        Self {
            launch,
            capacity: 16,
            startup: Duration::from_secs(10),
            admission: Duration::from_millis(AdmissionTimeout::default().milliseconds() as u64),
            cleanup: Duration::from_secs(5),
        }
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            (1..=256).contains(&self.capacity),
            "invalid registration capacity"
        );
        for value in [self.startup, self.cleanup] {
            ensure!(
                !value.is_zero() && value <= Duration::from_secs(60),
                "invalid registration deadline"
            );
        }
        let admission = AdmissionTimeout::new(self.admission.as_millis().try_into()?)?;
        ensure!(
            self.admission == Duration::from_millis(admission.milliseconds() as u64),
            "registration admission deadline must use whole milliseconds"
        );
        Ok(())
    }
}
pub struct Factory {
    settings: Settings,
    connector: Arc<Connector>,
    pool: Pool,
}
impl Factory {
    pub fn new(settings: Settings) -> Result<Self> {
        settings.validate()?;
        let connector = Arc::new(Connector::new(settings.launch.clone(), 1)?);
        let pool = Pool::new(settings.capacity)?;
        Ok(Self {
            settings,
            connector,
            pool,
        })
    }
    /// Pass owned duplicates of the session's listeners, video then control for
    /// display. This client never accepts on them. Arbitrary ports are not proof.
    pub fn start(&self, request: Request, listeners: Vec<TcpListener>) -> Result<Handle> {
        let model = Model::new(request.clone())?;
        let endpoints = Arc::new(Endpoints::validate(
            request.resource,
            listeners.into_iter().map(OwnedFd::from).collect(),
        )?);
        let admission = self.pool.reserve()?;
        let completion = admission.completion();
        let shared = Arc::new(Shared::new(model));
        let worker = worker::Worker::new(
            self.settings.clone(),
            self.connector.clone(),
            shared.clone(),
            endpoints.clone(),
        );
        let thread = std::thread::Builder::new()
            .name("blent-media-registration".into())
            .spawn(move || worker.run())?;
        admission.retain(Retained {
            thread: retirement_work::Thread::new(thread),
            shared: shared.clone(),
            _endpoints: endpoints,
        });
        Ok(Handle { shared, completion })
    }
}
pub struct Handle {
    shared: Arc<Shared>,
    completion: Completion,
}
impl Client for Handle {
    fn status(&self) -> Status {
        self.shared.state().model.status()
    }
    fn close(&self) {
        self.shared.update(Model::close);
        self.shared.wake.notify_one();
    }
    fn retired(&self) -> bool {
        self.completion.is_complete()
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.close();
    }
}
struct State {
    model: Model,
    closing: Option<Instant>,
    broker: Option<lifetime::Original>,
}
struct Shared {
    state: Mutex<State>,
    wake: Notify,
}
impl Shared {
    fn new(model: Model) -> Self {
        Self {
            state: Mutex::new(State {
                model,
                closing: None,
                broker: None,
            }),
            wake: Notify::new(),
        }
    }
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn update<T>(&self, operation: impl FnOnce(&mut Model) -> T) -> T {
        let mut state = self.state();
        let value = operation(&mut state.model);
        if matches!(
            state.model.status().phase,
            Phase::Closing | Phase::Failed | Phase::Retired
        ) {
            state.closing.get_or_insert_with(Instant::now);
        }
        value
    }
    fn fail(&self, failure: Failure) {
        self.update(|model| model.fail(failure));
    }
    fn deadline(&self, admission: Instant, cleanup: Duration) -> Option<Instant> {
        let state = self.state();
        if let Some(at) = state.closing {
            return Some(at + cleanup);
        }
        (state.model.status().phase != Phase::Active).then_some(admission)
    }
}
struct Retained {
    thread: retirement_work::Thread,
    shared: Arc<Shared>,
    _endpoints: Arc<Endpoints>,
}
impl Observe for Retained {
    fn poll_complete(&mut self) -> bool {
        let native = self.thread.poll_complete();
        if !native {
            return false;
        }
        let mut state = self.shared.state();
        // Only completed local work needs exit polling. Active/stuck native
        // threads retain ownership without an additional kernel poll per turn.
        let State { model, broker, .. } = &mut *state;
        if let Some(original) = broker {
            model.observe_broker_exit(original);
        }
        model.status().remote_retired
    }
}
#[cfg(test)]
mod tests;
