//! Linux adapter for the independent presentation port. One process-owned
//! factory reserves one worker until its thread and Tokio runtime really retire.
use super::client::{Connector, Launch};
use anyhow::{ensure, Result};
use blent_config::{
    connection::{
        control_client::{Client, Phase, Status, SubmitError},
        control_view::{Action, Assembler, Limits, View},
    },
    retirement::{Completion, Pool},
    retirement_work,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};
mod exchange;
mod worker;

#[derive(Clone)]
pub struct Settings {
    pub launch: Launch,
    pub startup: Duration,
    pub transaction: Duration,
    pub poll: Duration,
    pub queue: usize,
    pub limits: Limits,
}
impl Settings {
    pub fn new(launch: Launch) -> Self {
        Self {
            launch,
            startup: Duration::from_secs(10),
            transaction: Duration::from_secs(5),
            poll: Duration::from_millis(500),
            queue: 8,
            limits: Limits::default(),
        }
    }
    fn validate(&self) -> Result<()> {
        for value in [self.startup, self.transaction, self.poll] {
            ensure!(
                !value.is_zero() && value <= Duration::from_secs(60),
                "invalid broker client deadline"
            );
        }
        ensure!(
            (1..=32).contains(&self.queue),
            "invalid broker client command capacity"
        );
        Assembler::new(self.limits)?;
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
        Ok(Self {
            settings,
            connector,
            pool: Pool::new(1)?,
        })
    }
    pub fn start(&self) -> Result<Handle> {
        let admission = self.pool.reserve()?;
        let completion = admission.completion();
        let (commands, receiver) = mpsc::channel(self.settings.queue);
        let (stop, stopped) = oneshot::channel();
        let shared = Arc::new(Shared::new());
        let worker = worker::Worker::new(
            self.settings.clone(),
            self.connector.clone(),
            shared.clone(),
        );
        let thread = std::thread::Builder::new()
            .name("blent-pairing-client".into())
            .spawn(move || worker.run(receiver, stopped))?;
        admission.retain(retirement_work::Thread::new(thread));
        Ok(Handle {
            commands,
            stop: Mutex::new(Some(stop)),
            shared,
            completion,
        })
    }
}

pub struct Handle {
    commands: mpsc::Sender<Action>,
    stop: Mutex<Option<oneshot::Sender<()>>>,
    shared: Arc<Shared>,
    completion: Completion,
}
impl Client for Handle {
    fn status(&self) -> Status {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    fn submit(&self, action: Action) -> std::result::Result<(), SubmitError> {
        if self.shared.stopped.load(Ordering::Acquire) {
            return Err(SubmitError::Closed);
        }
        self.commands.try_send(action).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => SubmitError::Full,
            mpsc::error::TrySendError::Closed(_) => SubmitError::Closed,
        })
    }
    fn close(&self) {
        self.shared.stop();
        if let Some(stop) = self
            .stop
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            let _ = stop.send(());
        }
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

struct Shared {
    state: Mutex<Status>,
    stopped: AtomicBool,
}
impl Shared {
    fn new() -> Self {
        Self {
            state: Mutex::new(Status {
                phase: Phase::Connecting,
                view: None,
                message: None,
            }),
            stopped: AtomicBool::new(false),
        }
    }
    fn publish(&self, phase: Phase, view: Option<View>, message: Option<String>) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.stopped.load(Ordering::Acquire) && !matches!(phase, Phase::Closed | Phase::Failed) {
            return;
        }
        *state = Status {
            phase,
            view,
            message: message.map(|text| text.chars().take(512).collect()),
        };
    }
    fn stop(&self) {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !matches!(state.phase, Phase::Closed | Phase::Failed) {
            *state = Status {
                phase: Phase::Closing,
                view: None,
                message: None,
            };
        }
    }
    fn finish(&self, result: Result<()>) {
        match result {
            Ok(()) => self.publish(Phase::Closed, None, None),
            Err(error) => self.publish(Phase::Failed, None, Some(format!("{error:#}"))),
        }
    }
}

#[cfg(test)]
mod tests;
