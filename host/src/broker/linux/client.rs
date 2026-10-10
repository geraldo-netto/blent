//! On-demand per-user broker startup; local paths never come from peer messages.
use super::{async_io::AsyncSocket, Socket};
use anyhow::{ensure, Context, Result};
use std::{io, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    sync::Semaphore,
    time::{timeout_at, Instant},
};

#[derive(Clone)]
pub struct Launch {
    pub executable: PathBuf,
    pub runtime: PathBuf,
    pub state: PathBuf,
    pub helper: PathBuf,
}
impl Launch {
    fn validate(&self) -> Result<()> {
        ensure!(
            [&self.executable, &self.runtime, &self.state, &self.helper]
                .iter()
                .all(|path| path.is_absolute()),
            "broker launch paths must be absolute"
        );
        Ok(())
    }
    fn spawn(&self) -> Result<()> {
        let mut command = std::process::Command::new(&self.executable);
        command
            .args(["--runtime"])
            .arg(&self.runtime)
            .arg("--state")
            .arg(&self.state)
            .arg("--helper")
            .arg(&self.helper)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        // The elected broker survives the requesting client's exit. Its own
        // lifecycle, not this startup request, governs attached resources.
        blent_config::commands::spawn_reaped(&mut command).context("start local USB broker")?;
        Ok(())
    }
    fn connect(&self) -> io::Result<AsyncSocket> {
        AsyncSocket::new(Socket::connect(
            &self.runtime.join("broker.sock"),
            unsafe { libc::geteuid() },
        )?)
    }
}

/// Share one Connector per application owner. Stuck native process creation
/// keeps its starter permit until the blocking operation actually returns;
/// cancelled or timed-out client waits cannot create replacement starter work.
pub struct Connector {
    launch: Arc<Launch>,
    starters: Arc<Semaphore>,
}
impl Connector {
    pub fn new(launch: Launch, starters: usize) -> Result<Self> {
        launch.validate()?;
        ensure!(
            (1..=64).contains(&starters),
            "invalid broker starter capacity"
        );
        Ok(Self {
            launch: Arc::new(launch),
            starters: Arc::new(Semaphore::new(starters)),
        })
    }
    pub async fn connect(&self, timeout: Duration) -> Result<AsyncSocket> {
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(60),
            "invalid broker startup deadline"
        );
        let deadline = Instant::now() + timeout;
        match self.launch.connect() {
            Ok(socket) => return Ok(socket),
            Err(error) if missing(&error) => self.start(deadline).await?,
            Err(error) if busy(&error) => (),
            Err(error) => return Err(error.into()),
        }
        self.wait(deadline).await
    }
    async fn start(&self, deadline: Instant) -> Result<()> {
        let launch = self.launch.clone();
        self.start_with(deadline, move || launch.spawn()).await
    }
    async fn start_with(
        &self,
        deadline: Instant,
        operation: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> Result<()> {
        let permit = timeout_at(deadline, self.starters.clone().acquire_owned()).await??;
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation()
        });
        timeout_at(deadline, task).await???;
        Ok(())
    }
    async fn wait(&self, deadline: Instant) -> Result<AsyncSocket> {
        loop {
            ensure!(Instant::now() < deadline, "broker startup deadline expired");
            match self.launch.connect() {
                Ok(socket) => return Ok(socket),
                Err(error) if missing(&error) || busy(&error) => (),
                Err(error) => return Err(error.into()),
            }
            timeout_at(deadline, tokio::time::sleep(Duration::from_millis(20))).await?;
        }
    }
}
fn missing(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}
fn busy(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::WouldBlock
}

#[cfg(test)]
mod tests;
