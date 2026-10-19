//! CLI adapter for the local experimental broker, independent of media clients.
use super::server::{Server, Settings};
use anyhow::Result;
use clap::Parser;
use std::{ffi::OsString, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    name = "blent-usb-broker",
    about = "Experimental per-user USB broker; production media integration pending"
)]
pub struct Options {
    #[arg(long)]
    runtime: PathBuf,
    #[arg(long)]
    state: PathBuf,
    #[arg(long)]
    helper: PathBuf,
    #[arg(long, default_value_t = 16)]
    clients: usize,
    #[arg(long, default_value_t = 4)]
    links: usize,
    #[arg(long, default_value_t = 64)]
    devices: usize,
    #[arg(long, default_value_t = 64)]
    registrations: usize,
    #[arg(long, default_value_t = 64)]
    trust_peers: usize,
    #[arg(long, default_value_t = 65536)]
    block_bytes: usize,
    #[arg(long, default_value_t = 262144)]
    channel_bytes: u32,
    #[arg(long, default_value_t = 64)]
    queued_messages: usize,
    #[arg(long, default_value_t = 65536)]
    quantum: u32,
    #[arg(long, default_value_t = 5000)]
    io_timeout_ms: u64,
    #[arg(long, default_value_t = 60000)]
    setup_ms: i64,
    #[arg(long, default_value_t = 120000)]
    approval_ms: i64,
    #[arg(long, default_value_t = 5000)]
    retirement_timeout_ms: u64,
    #[arg(long, default_value_t = blent_config::connection::resources::AdmissionTimeout::default().milliseconds())]
    resource_timeout_ms: i64,
    #[arg(long, default_value_t = blent_config::connection::handshake::Timeout::default().duration().as_millis() as u64)]
    tls_timeout_ms: u64,
}
impl Options {
    pub fn settings(self) -> Settings {
        let mut settings = Settings::new(self.runtime, self.state, self.helper);
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
        settings
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
    serve(options.settings(), stop_signal()).await
}
pub async fn serve(
    settings: Settings,
    stop: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    serve_opened(Server::open(settings).await?, stop).await
}
/// Coordinate one admitted backend owner; startup and policy validation precede
/// this workflow. Shutdown keeps the same server's immutable observation budget.
pub(super) async fn serve_opened(
    mut server: Server,
    stop: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    let result = tokio::select! {
        result = server.run() => result,
        stopped = stop => stopped,
    };
    server.shutdown_configured().await?;
    result
}
async fn stop_signal() -> Result<()> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result?,
        _ = terminate.recv() => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
