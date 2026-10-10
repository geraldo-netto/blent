//! Authenticated runtime plus independent native-owner retirement evidence.
use super::session::{Limits, Session};
use crate::accessory::{setup::Verified, Owner, Resource, Stream};
use anyhow::{ensure, Result};
use blent_config::{
    connection::pairing::{Lease, Pairing},
    retirement::Completion,
};

pub struct Connection {
    session: Session<Box<dyn Stream>>,
    owner: Option<Box<dyn Owner>>,
    completion: Completion,
}
impl Connection {
    /// Capture the native Completion before discovery, and retain it if any
    /// setup step fails. A failed constructor never proves helper retirement.
    pub fn new(
        verified: Verified,
        pairing: &Pairing,
        lease: &Lease,
        limits: Limits,
    ) -> Result<Self> {
        ensure!(
            pairing.current(lease)
                && verified.ticket == lease.ticket
                && verified.peer == lease.ticket.peer,
            "stale authenticated setup"
        );
        let Resource { stream, owner } = verified.resource;
        let completion = owner.completion();
        Ok(Self {
            session: Session::new(stream, lease.ticket.clone(), limits)?,
            owner: Some(owner),
            completion,
        })
    }
    pub fn session(&mut self) -> &mut Session<Box<dyn Stream>> {
        &mut self.session
    }
    pub fn completion(&self) -> Completion {
        self.completion.clone()
    }
    /// Disconnect media first. Each local bridge still reports its own actual
    /// retirement separately. This future belongs to one link owner, never the
    /// shared broker dispatcher; cancellation does not count as completion.
    pub async fn retire(&mut self) -> Result<()> {
        let disconnected = self.session.disconnect();
        if let Some(owner) = self.owner.take() {
            owner.retire().await?;
        }
        ensure!(
            self.completion.is_complete(),
            "native link still owned; keep quarantined"
        );
        disconnected
    }
}
#[cfg(test)]
mod tests;
