//! CLI adapter for the local experimental broker, independent of media clients.
use super::server::{Server, Settings};
use crate::broker::capacity;
use anyhow::{Context, Result};
use clap::Parser;
use std::{ffi::OsString, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    name = "blent-usb-broker",
    about = "Experimental per-user USB broker for paired display and media"
)]
pub struct Options {
    #[arg(long, required_unless_present = "reset", conflicts_with = "reset")]
    runtime: Option<PathBuf>,
    #[arg(long)]
    state: PathBuf,
    #[arg(long, required_unless_present = "reset", conflicts_with = "reset")]
    helper: Option<PathBuf>,
    /// Offline repair; identity reset also removes all saved tablet approvals.
    #[arg(long, value_enum, requires = "confirm_reset")]
    reset: Option<repair::Reset>,
    /// Confirm destruction of the selected saved state.
    #[arg(long, requires = "reset")]
    confirm_reset: bool,
    #[arg(long, conflicts_with = "reset", default_value_t = capacity::DEFAULT_CLIENTS)]
    clients: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = capacity::DEFAULT_LINKS)]
    links: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = 64)]
    devices: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = capacity::DEFAULT_REGISTRATIONS)]
    registrations: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = 64)]
    trust_peers: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = 65536)]
    block_bytes: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = 262144)]
    channel_bytes: u32,
    #[arg(long, conflicts_with = "reset", default_value_t = 64)]
    queued_messages: usize,
    #[arg(long, conflicts_with = "reset", default_value_t = 65536)]
    quantum: u32,
    #[arg(long, conflicts_with = "reset", default_value_t = 5000)]
    io_timeout_ms: u64,
    #[arg(long, conflicts_with = "reset", default_value_t = 60000)]
    setup_ms: i64,
    #[arg(long, conflicts_with = "reset", default_value_t = 120000)]
    approval_ms: i64,
    #[arg(long, conflicts_with = "reset", default_value_t = 5000)]
    retirement_timeout_ms: u64,
    #[arg(long, conflicts_with = "reset", default_value_t = blent_config::connection::resources::AdmissionTimeout::default().milliseconds())]
    resource_timeout_ms: i64,
    #[arg(long, conflicts_with = "reset", default_value_t = blent_config::connection::handshake::Timeout::default().duration().as_millis() as u64)]
    tls_timeout_ms: u64,
}
impl Options {
    pub fn settings(self) -> Result<Settings> {
        let mut settings = Settings::new(
            self.runtime
                .context("--runtime is required to run the broker")?,
            self.state,
            self.helper
                .context("--helper is required to run the broker")?,
        );
        settings.control.clients = self.clients;
        settings.control.links = self.links;
        settings.control.devices = self.devices;
        settings.control.setup_ms = self.setup_ms;
        settings.control.approval_ms = self.approval_ms;
        settings.registrations = self.registrations;
        settings.trust_peers = self.trust_peers;
        settings.block = self.block_bytes;
        settings.session.channel_bytes = self.channel_bytes;
        settings.session.queued_messages = self.queued_messages;
        settings.session.quantum = self.quantum;
        settings.session.io_timeout = Duration::from_millis(self.io_timeout_ms);
        settings.retirement_timeout = Duration::from_millis(self.retirement_timeout_ms);
        settings.resource_timeout_ms = self.resource_timeout_ms;
        settings.tls_timeout = Duration::from_millis(self.tls_timeout_ms);
        Ok(settings)
    }
}
pub async fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
    let options = match Options::try_parse_from(arguments) {
        Ok(options) => options,
        Err(error) if error.use_stderr() => return Err(error.into()),
        Err(help) => {
            help.print()?;
            return Ok(());
        }
    };
    let stop = stop_signal()?;
    if let Some(selected) = options.reset {
        return repair::reset(&options.state, selected);
    }
    serve(options.settings()?, stop).await
}
pub async fn serve(
    settings: Settings,
    stop: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    #[cfg(test)]
    startup_tests::pause().await;
    serve_opened(Server::open(settings).await?, stop).await
}
/// Coordinate one admitted backend owner; startup and policy validation precede
/// this workflow. Shutdown keeps the same server's immutable observation budget.
pub(super) async fn serve_opened(
    mut server: Server,
    stop: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    let result = tokio::select! {
        biased;
        stopped = stop => stopped,
        result = server.run() => result,
    };
    server.shutdown_configured().await?;
    result
}
fn stop_signal() -> Result<impl std::future::Future<Output = Result<()>>> {
    // Register eagerly: constructing an async fn would defer registration until
    // after Server::open. Streams retain a stop while original startup settles.
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    Ok(async move {
        tokio::select! {
            _ = interrupt.recv() => (),
            _ = terminate.recv() => (),
        }
        Ok(())
    })
}

mod repair;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod startup_tests;

#[cfg(test)]
mod repair_tests;
