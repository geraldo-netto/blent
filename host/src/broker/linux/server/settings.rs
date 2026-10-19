use super::*;
use crate::broker::control;
use std::{os::unix::fs::DirBuilderExt, path::PathBuf};

#[derive(Clone)]
pub struct Settings {
    pub runtime: PathBuf,
    pub state: PathBuf,
    pub helper: PathBuf,
    pub control: control::Limits,
    pub session: session::Limits,
    pub registrations: usize,
    pub trust_peers: usize,
    pub block: usize,
    pub retirement_timeout: Duration,
    pub resource_timeout_ms: i64,
    pub tls_timeout: Duration,
}
impl Settings {
    pub fn new(runtime: PathBuf, state: PathBuf, helper: PathBuf) -> Self {
        Self {
            runtime,
            state,
            helper,
            control: control::Limits::default(),
            session: session::Limits::default(),
            registrations: 64,
            trust_peers: 64,
            block: 65536,
            retirement_timeout: Duration::from_secs(5),
            tls_timeout: blent_config::connection::handshake::Timeout::default().duration(),
            resource_timeout_ms: blent_config::connection::resources::AdmissionTimeout::default()
                .milliseconds(),
        }
    }
    pub(super) fn validate(&self) -> Result<()> {
        blent_config::retirement_limits::observation_budget(self.retirement_timeout)?;
        blent_config::connection::resources::AdmissionTimeout::new(self.resource_timeout_ms)?;
        blent_config::connection::handshake::Timeout::new(self.tls_timeout)?;
        ensure!(
            [&self.runtime, &self.state, &self.helper]
                .iter()
                .all(|path| path.is_absolute()),
            "broker paths must be absolute"
        );
        ensure!(
            (1..=1024).contains(&self.trust_peers),
            "invalid saved peer capacity"
        );
        let peer = PeerId::parse("0".repeat(64))?;
        let _ = Control::new(peer, Vec::new(), self.control)?;
        let _ = RouteClaims::new(self.registrations)?;
        let _ = crate::broker::bridges::Bridges::new(self.block, self.session.io_timeout)?;
        blent_config::connection::window::validate(self.session.channel_bytes)?;
        // Reuse the session's complete bounds without connecting native I/O.
        let (stream, _) = tokio::io::duplex(1);
        let ticket = blent_config::connection::pairing::Ticket {
            epoch: 1,
            peer: PeerId::parse("0".repeat(64))?,
        };
        let _ = session::Session::new(stream, ticket, self.session)?;
        Ok(())
    }
    /// Map one validated backend snapshot into its original link's narrow view.
    pub(super) fn link(
        &self,
        identity: std::sync::Arc<crate::secure_connection::Identity>,
        origin: Instant,
    ) -> Result<link::Settings> {
        Ok(link::Settings {
            identity,
            limits: self.session,
            block: self.block,
            setup_timeout: Duration::from_millis(self.control.setup_ms as u64),
            approval_timeout: Duration::from_millis(self.control.approval_ms as u64),
            resource_timeout: blent_config::connection::resources::AdmissionTimeout::new(
                self.resource_timeout_ms,
            )?,
            tls_timeout: blent_config::connection::handshake::Timeout::new(self.tls_timeout)?,
            origin,
        })
    }
    pub(super) fn directories(&self) -> Result<()> {
        for path in [&self.runtime, &self.state] {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
