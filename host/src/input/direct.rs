//! T673: negotiated non-stylus sessions. Native resources live on one owned worker.
use super::{
    backend::{InputBackend, InputSink, PenSample},
    InputConfig,
};
use anyhow::{ensure, Context, Result};
use blent_config::{
    direct_input::{Adapter, Config, Event, Mode, Phase, Session},
    input_mapping::Snapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
    thread,
};
use tokio::sync::watch;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Negotiate { version: u32 },
    Select { mode: Mode },
    Event { event: Event },
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub protocol: u32,
    pub monitor: String,
    pub mode: Mode,
    pub touch: bool,
    pub mouse: bool,
    pub negotiated: bool,
}

pub trait Environment: 'static {
    type Device: Adapter;
    fn snapshot(&self) -> Result<Snapshot>;
    fn create(&self, mode: Mode) -> Result<Self::Device>;
    fn save(&self, config: &Config) -> Result<()>;
}

mod state;
mod worker;

struct Authority(watch::Sender<bool>);
impl Authority {
    fn new() -> Arc<Self> {
        Arc::new(Self(watch::channel(false).0))
    }
    fn cancel(&self) {
        self.0.send_replace(true);
    }
    fn cancelled(&self) -> bool {
        *self.0.borrow()
    }
}
const QUEUED_COMMANDS: usize = 16;
const OUTSTANDING_COMMANDS: usize = QUEUED_COMMANDS + 1;

struct Gate {
    current: Arc<Authority>,
    status: Status,
    closed: bool,
}
struct Message {
    command: Command,
    authority: Arc<Authority>,
    reply: Option<tokio::sync::oneshot::Sender<Result<()>>>,
    _admission: Arc<tokio::sync::OwnedSemaphorePermit>,
}
enum Request {
    Command(Message),
    Wake,
}

pub struct Backend {
    sink: Arc<Sink>,
    completion: blent_config::retirement::Completion,
}
struct Sink {
    sender: mpsc::SyncSender<Request>,
    gate: Arc<Mutex<Gate>>,
    commands: Arc<tokio::sync::Semaphore>,
}

impl Backend {
    pub fn new<E: Environment>(
        config: Config,
        touch: bool,
        mouse: bool,
        factory: impl FnOnce() -> E + Send + 'static,
    ) -> Result<Self> {
        let admission = reserve()?;
        let status = Status {
            protocol: 1,
            monitor: config.monitor,
            mode: config.mode,
            touch,
            mouse,
            negotiated: false,
        };
        let gate = Arc::new(Mutex::new(Gate {
            current: Authority::new(),
            status: status.clone(),
            closed: false,
        }));
        let (sender, receiver) = mpsc::sync_channel(QUEUED_COMMANDS);
        let owned = gate.clone();
        let handle = thread::Builder::new()
            .name("blent-direct-input".into())
            .spawn(move || {
                worker::serve(
                    state::State {
                        environment: factory(),
                        status,
                        session: None,
                    },
                    receiver,
                    owned,
                );
            })?;
        let completion = admission.completion();
        admission.retain(blent_config::retirement_work::Thread::new(handle));
        Ok(Self {
            sink: Arc::new(Sink {
                sender,
                gate,
                commands: Arc::new(tokio::sync::Semaphore::new(OUTSTANDING_COMMANDS)),
            }),
            completion,
        })
    }
    /// Exact original worker exit; native release acceptance remains an adapter obligation.
    pub fn completion(&self) -> blent_config::retirement::Completion {
        self.completion.clone()
    }
}

fn reserve() -> Result<blent_config::retirement::Admission> {
    use blent_config::retirement::Pool;
    static POOL: std::sync::OnceLock<std::io::Result<Pool>> = std::sync::OnceLock::new();
    let pool = POOL
        .get_or_init(|| {
            Pool::new(blent_config::retirement_limits::configured(
                "BLENT_INPUT_WORKER_LIMIT",
                16,
                4096,
            )?)
        })
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    Ok(pool.reserve()?)
}

/// Dropping a pending reply revokes only its original authority, never a successor.
struct Pending {
    wake: mpsc::SyncSender<Request>,
    _admission: Arc<tokio::sync::OwnedSemaphorePermit>,
    authority: Arc<Authority>,
    completed: bool,
}
impl Pending {
    fn complete(&mut self) {
        self.completed = true;
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        if !self.completed {
            self.authority.cancel();
            let _ = self.wake.try_send(Request::Wake);
        }
    }
}
impl Sink {
    fn admit(
        &self,
        command: Command,
        reply: Option<tokio::sync::oneshot::Sender<Result<()>>>,
    ) -> Result<(Arc<Authority>, Arc<tokio::sync::OwnedSemaphorePermit>)> {
        let admission = Arc::new(
            self.commands
                .clone()
                .try_acquire_owned()
                .context("Input command completion capacity unavailable")?,
        );
        let mut gate = self.gate.lock().unwrap();
        if gate.current.cancelled() {
            gate.current = Authority::new();
            gate.status.negotiated = false;
        }
        let authority = gate.current.clone();
        self.sender
            .try_send(Request::Command(Message {
                command,
                authority: authority.clone(),
                reply,
                _admission: admission.clone(),
            }))
            .map_err(|_| anyhow::anyhow!("Input command capacity unavailable"))?;
        Ok((authority, admission))
    }
    fn request(&self, command: Command) -> Result<super::backend::DirectReply> {
        let (reply, response) = tokio::sync::oneshot::channel();
        let (authority, admission) = self.admit(command, Some(reply))?;
        let mut cancelled = authority.0.subscribe();
        let mut pending = Pending {
            wake: self.sender.clone(),
            _admission: admission,
            authority,
            completed: false,
        };
        Ok(Box::pin(async move {
            ensure!(!*cancelled.borrow(), "Input command revoked");
            let result = tokio::select! {
                biased;
                _ = cancelled.changed() => Err(anyhow::anyhow!("Input command revoked")),
                result = response => result.context("Input worker stopped")?,
            };
            pending.complete();
            result
        }))
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        let mut gate = self.gate.lock().unwrap();
        gate.closed = true;
        gate.current.cancel();
        gate.status.negotiated = false;
        let _ = self.sender.try_send(Request::Wake);
    }
}
impl InputSink for Sink {
    fn release_all(&self) {
        let mut gate = self.gate.lock().unwrap();
        gate.current.cancel();
        gate.status.negotiated = false;
        let _ = self.sender.try_send(Request::Wake);
    }
    fn touch(&self, (x, y, _): (f64, f64, f64), action: u8, slot: u8) {
        if let Err(error) =
            legacy_touch(x, y, action, slot).and_then(|command| self.admit(command, None))
        {
            self.release_all();
            tracing::warn!("Touch rejected: {error}");
        }
    }
    fn pen(&self, _: PenSample, _: bool) {
        self.release_all();
    }
    fn direct(&self, command: Command) -> Result<super::backend::DirectReply> {
        self.request(command)
    }
    fn direct_status(&self) -> Option<Status> {
        let gate = self.gate.lock().unwrap();
        let mut status = gate.status.clone();
        status.negotiated = status.negotiated && !gate.current.cancelled();
        Some(status)
    }
}

pub(super) fn legacy_touch(x: f64, y: f64, action: u8, slot: u8) -> Result<Command> {
    let phase = match action {
        0 => Phase::Down,
        1 => Phase::Up,
        2 => Phase::Move,
        _ => anyhow::bail!("Invalid touch action"),
    };
    Ok(Command::Event {
        event: Event::Touch { x, y, slot, phase },
    })
}
impl InputBackend for Backend {
    fn sink(&self) -> Arc<dyn InputSink> {
        self.sink.clone()
    }
    fn native_completion(&self) -> Option<blent_config::retirement::Completion> {
        Some(self.completion())
    }
    fn follow(
        &self,
        mut tablet: watch::Receiver<bool>,
        mut mode: watch::Receiver<bool>,
        _: InputConfig,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let sink = self.sink.clone();
        Box::pin(async move {
            loop {
                let changed = tokio::select! {
                    result = tablet.changed() => { if result.is_ok() && *tablet.borrow_and_update() { continue; } result },
                    result = mode.changed() => result
                };
                sink.release_all();
                if changed.is_err() {
                    break;
                }
            }
        })
    }
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::native;
#[cfg(test)]
mod tests;
