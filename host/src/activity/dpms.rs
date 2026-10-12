//! A blocked Xlib call owns an isolated process; async cancellation is no proof.
use super::*;
use blent_config::{
    commands::OwnedChild, host_activity::DisplayObservation, retirement::Completion,
};
use std::{ffi::OsString, path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{ChildStdout, Command},
    sync::oneshot,
};
pub(super) struct Program {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
}
impl Program {
    pub fn current(settings: Settings) -> Result<Self> {
        settings.validate()?;
        let interval = (settings.deadline / 4)
            .max(Duration::from_nanos(1))
            .min(Duration::from_millis(250));
        Ok(Self {
            executable: std::env::current_exe()?,
            arguments: vec![
                "--blent-activity-dpms".into(),
                interval.as_nanos().to_string().into(),
                "0".into(),
            ],
        })
    }
}
struct Startup {
    task: Option<Task>,
    child: oneshot::Receiver<OwnedChild>,
    native: Arc<Mutex<Vec<Completion>>>,
}
impl Startup {
    fn begin(program: Program) -> Result<Self> {
        ensure!(
            program.executable.is_absolute(),
            "invalid DPMS helper executable"
        );
        Self::launch(move || {
            let mut command = Command::new(program.executable);
            command
                .args(program.arguments)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            Ok(OwnedChild::spawn(&mut command)?)
        })
    }
    fn launch(create: impl FnOnce() -> Result<OwnedChild> + Send + 'static) -> Result<Self> {
        let (send, child) = oneshot::channel();
        let native = Arc::new(Mutex::new(Vec::new()));
        let proof = native.clone();
        let task = Task::spawn_blocking(move || {
            let child = create()?;
            // Capture original native ownership BEFORE any cancellable result.
            proof.lock().unwrap().push(child.completion());
            let _ = send.send(child);
            Ok(())
        })?;
        native.lock().unwrap().push(task.completion());
        Ok(Self {
            task: Some(task),
            child,
            native,
        })
    }
    async fn take(&mut self, deadline: Duration) -> Option<OwnedChild> {
        tokio::time::timeout(deadline, &mut self.child)
            .await
            .ok()?
            .ok()
    }
    async fn joined(&mut self) {
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
struct Child {
    process: OwnedChild,
    reader: BufReader<ChildStdout>,
}
impl Child {
    fn new(mut process: OwnedChild) -> Result<Self> {
        let output = process
            .take_stdout()
            .ok_or_else(|| anyhow::anyhow!("DPMS helper output missing"))?;
        Ok(Self {
            process,
            reader: BufReader::with_capacity(257, output),
        })
    }
    async fn sample(&mut self, deadline: Duration) -> Result<DisplayObservation> {
        let mut bytes = Vec::with_capacity(257);
        let read = tokio::time::timeout(
            deadline,
            (&mut self.reader).take(257).read_until(b'\n', &mut bytes),
        )
        .await??;
        ensure!(
            read > 0 && read <= 256 && bytes.last() == Some(&b'\n'),
            "invalid DPMS helper frame extent"
        );
        DisplayObservation::decode(&bytes)
    }
}
pub(super) struct Probe {
    startup: Startup,
    child: Option<Child>,
    initial: DisplayObservation,
    deadline: Duration,
    live: bool,
}
impl Probe {
    pub async fn start(program: Program, settings: Settings) -> Result<Self> {
        settings.validate()?;
        Self::from_startup(Startup::begin(program)?, settings).await
    }
    async fn from_startup(mut startup: Startup, settings: Settings) -> Result<Self> {
        let mut child = startup
            .take(settings.deadline)
            .await
            .and_then(|child| Child::new(child).ok());
        let first = match &mut child {
            Some(child) => child.sample(settings.deadline).await.ok(),
            None => None,
        };
        let initial = first.unwrap_or(DisplayObservation {
            supported: false,
            on: None,
        });
        Ok(Self {
            startup,
            child,
            initial,
            deadline: settings.deadline,
            live: initial.supported && first.is_some(),
        })
    }
    async fn run(mut self, shared: Arc<Shared>) -> Result<()> {
        self.startup.joined().await;
        let outcome = self.watch(&shared).await;
        if let Err(error) = outcome {
            tracing::warn!(%error, "original X11 power observer lost");
        }
        let _ = shared.update(native::Patch::Lost(Capabilities {
            display: self.initial.supported,
            ..Capabilities::default()
        }));
        if let Some(child) = self.child.take() {
            let _ = child.process.retire(Duration::ZERO).await;
        }
        // A timed-out creator may transfer a child only after actual startup returns.
        if let Ok(child) = self.startup.child.try_recv() {
            let _ = child.retire(Duration::ZERO).await;
        }
        Ok(())
    }
    async fn watch(&mut self, shared: &Shared) -> Result<()> {
        if !self.live {
            return Ok(());
        }
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("missing original DPMS helper"))?;
        loop {
            let wake = shared.wake.notified();
            tokio::pin!(wake);
            wake.as_mut().enable();
            if shared.stopped.load(Ordering::Acquire) {
                return Ok(());
            }
            let sample = tokio::select! { biased;
                _ = &mut wake => return Ok(()),
                sample = child.sample(self.deadline) => sample?,
            };
            ensure!(sample.supported, "original DPMS capability withdrawn");
            shared.update(native::Patch::Display(sample.on))?;
        }
    }
}
impl Backend for Probe {
    fn include(&self, caps: &mut Capabilities, state: &mut HostState) {
        caps.display |= self.initial.supported;
        if self.initial.supported {
            state.display_on = self.initial.on;
        }
    }
    fn spawn(self: Box<Self>, shared: Arc<Shared>) -> Result<Work> {
        let native = self.startup.native.clone();
        let task = Task::spawn(async move { self.run(shared).await })?;
        Ok(Work {
            task: Some(task),
            native,
        })
    }
}
#[cfg(test)]
mod tests;
