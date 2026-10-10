//! Portable desktop pairing presentation. A complete local-broker snapshot is
//! descriptive, never peer proof. UI actions retain their originating broker
//! lifetime and are revalidated immediately before transport submission.
use super::{
    control::{Command, Notice, State},
    pairing::{PeerId, Ticket},
};
use anyhow::{ensure, Result};
use std::{collections::BTreeMap, sync::Arc};
mod snapshot;
pub use snapshot::{Assembler, Device, Limits, Link, Snapshot};

#[derive(Clone)]
pub struct View {
    session: Arc<()>,
    snapshot: Arc<Snapshot>,
}
impl View {
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn prepare(&self, command: Command) -> Result<Action> {
        allowed(&self.snapshot, &command)?;
        Ok(Action {
            session: self.session.clone(),
            computer: self.snapshot.computer().clone(),
            command,
        })
    }
}

/// Bounded command metadata only: it does not retain an old device/trust list.
#[derive(Clone)]
pub struct Action {
    session: Arc<()>,
    computer: PeerId,
    command: Command,
}

pub struct Model {
    session: Arc<()>,
    current: Option<View>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            session: Arc::new(()),
            current: None,
        }
    }
}
impl Model {
    /// Call on every transport loss/replacement, even when the next broker has
    /// identical persisted identity, revision, connection and ticket counters.
    pub fn reset(&mut self) {
        self.session = Arc::new(());
        self.current = None;
    }
    pub fn view(&self) -> Option<View> {
        self.current.clone()
    }
    pub fn publish(&mut self, snapshot: Snapshot) -> Result<View> {
        if let Err(error) = self.check_snapshot(&snapshot) {
            self.reset();
            return Err(error);
        }
        let view = View {
            session: self.session.clone(),
            snapshot: Arc::new(snapshot),
        };
        self.current = Some(view.clone());
        Ok(view)
    }
    fn check_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        if let Some(view) = &self.current {
            let old = view.snapshot();
            ensure!(
                snapshot.computer() == old.computer(),
                "broker identity changed within connection"
            );
            ensure!(
                snapshot.revision() >= old.revision(),
                "broker snapshot went backwards"
            );
            ensure!(
                snapshot.revision() != old.revision() || snapshot == old,
                "conflicting broker revision"
            );
        }
        Ok(())
    }
    pub fn command<'a>(&self, action: &'a Action) -> Result<&'a Command> {
        ensure!(
            Arc::ptr_eq(&self.session, &action.session),
            "action belongs to another broker lifetime"
        );
        let view = self
            .current
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("broker snapshot unavailable"))?;
        ensure!(
            &action.computer == view.snapshot.computer(),
            "displayed computer changed"
        );
        allowed(&view.snapshot, &action.command)?;
        Ok(&action.command)
    }
}

fn allowed(snapshot: &Snapshot, command: &Command) -> Result<()> {
    command.encode()?;
    match command {
        Command::Inspect => Ok(()),
        Command::Connect { revision, target } => {
            ensure!(
                *revision == snapshot.revision() && snapshot.devices().contains_key(target),
                "stale USB device selection"
            );
            Ok(())
        }
        Command::Decide {
            connection, ticket, ..
        } => {
            let link = matching(snapshot, *connection, &Some(ticket.clone()))?;
            ensure!(
                link.state == State::Pending,
                "pairing choice is no longer pending"
            );
            Ok(())
        }
        Command::Disconnect { connection, ticket } => {
            let link = matching(snapshot, *connection, ticket)?;
            ensure!(
                !matches!(link.state, State::Closing | State::Quarantined),
                "connection already closing"
            );
            Ok(())
        }
        Command::Revoke { peer } => {
            ensure!(
                known(snapshot, peer),
                "computer did not display this tablet"
            );
            Ok(())
        }
    }
}
fn matching<'a>(
    snapshot: &'a Snapshot,
    connection: u64,
    ticket: &Option<Ticket>,
) -> Result<&'a Link> {
    let link = snapshot
        .links()
        .get(&connection)
        .ok_or_else(|| anyhow::anyhow!("connection no longer displayed"))?;
    ensure!(&link.ticket == ticket, "displayed tablet ticket changed");
    Ok(link)
}
fn known(snapshot: &Snapshot, peer: &PeerId) -> bool {
    snapshot.saved().contains_key(peer.as_str())
        || snapshot.links().values().any(|link| {
            link.ticket
                .as_ref()
                .is_some_and(|ticket| &ticket.peer == peer)
        })
}

#[cfg(test)]
mod tests;
