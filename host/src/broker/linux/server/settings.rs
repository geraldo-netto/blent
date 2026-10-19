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
        }
    }
    pub(super) fn validate(&self) -> Result<()> {
        blent_config::retirement_limits::observation_budget(self.retirement_timeout)?;
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
