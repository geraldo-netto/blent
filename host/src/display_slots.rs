//! T917: one portable display-capacity owner for legacy and authenticated peers.
//! Selection is local user intent, not authentication or resource admission.
//! Native adapters must retain every ticket until route AND session retirement.
use anyhow::{ensure, Result};
use blent_config::connection::pairing::PeerId;

mod contract;
pub use contract::{Owner, Plan, Reservations, Ticket};

struct Entry {
    owner: Owner,
    ticket: Ticket,
    closing: bool,
}
/// A single instance owns the host's configured 1..=4 display slots. Tickets are
/// local to this owner, never wire values. Small bounded scans avoid duplicate
/// indexes and keep all decisions independent of OS handles and capture types.
pub struct Slots {
    entries: Vec<Option<Entry>>,
    revision: u64,
    selected: Option<PeerId>,
    disabled: Vec<Owner>,
}
impl Slots {
    pub fn new(capacity: u32) -> Result<Self> {
        ensure!((1..=4).contains(&capacity), "invalid display capacity");
        Ok(Self {
            entries: (0..capacity).map(|_| None).collect(),
            revision: 0,
            selected: None,
            disabled: vec![],
        })
    }
    /// The connection adapter supplies an explicitly selected peer. Clearing or
    /// changing intent never releases an existing native resource's ticket.
    pub fn select(&mut self, peer: Option<PeerId>) {
        self.selected = peer;
    }

    pub fn enabled(&self, owner: &Owner) -> bool {
        !self.disabled.contains(owner)
    }

    pub fn set_enabled(&mut self, owner: Owner, enabled: bool) -> Result<()> {
        owner.target().identity()?;
        if enabled {
            self.disabled.retain(|other| other != &owner);
        } else if !self.disabled.contains(&owner) {
            ensure!(
                self.disabled.len() < 1024,
                "display intent capacity exceeded"
            );
            self.disabled.push(owner);
        }
        Ok(())
    }

    pub fn assignments(&self) -> Vec<blent_config::display_control::Assignment> {
        use blent_config::display_control::{Assignment, Phase};
        self.entries
            .iter()
            .flatten()
            .map(|entry| Assignment {
                target: entry.owner.target(),
                lease: entry.ticket.lease(),
                phase: if entry.closing {
                    Phase::Stopping
                } else {
                    Phase::Starting
                },
            })
            .collect()
    }
    pub fn owner(&self, ticket: Ticket) -> Option<&Owner> {
        self.entry(ticket).map(|entry| &entry.owner)
    }
    /// Reservation lifecycle only; an open ticket is not media admission.
    pub fn closing(&self, ticket: Ticket) -> Option<bool> {
        self.entry(ticket).map(|entry| entry.closing)
    }
    pub fn ticket(&self, owner: &Owner) -> Option<Ticket> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| &entry.owner == owner)
            .map(|entry| entry.ticket)
    }
    fn entry(&self, ticket: Ticket) -> Option<&Entry> {
        self.entries
            .get(ticket.instance as usize)?
            .as_ref()
            .filter(|entry| entry.ticket == ticket)
    }
    fn target(&self) -> Option<u32> {
        if self.revision == u64::MAX {
            return None;
        }
        if let Some(slot) = self.entries.iter().position(Option::is_none) {
            return Some(slot as u32);
        }
        // A retirement already underway can provide capacity. Do not evict
        // additional working displays while that cleanup remains quarantined.
        let retiring = self
            .entries
            .iter()
            .flatten()
            .rev()
            .find(|entry| entry.closing);
        retiring
            .or_else(|| {
                self.entries
                    .iter()
                    .flatten()
                    .rev()
                    .find(|entry| matches!(entry.owner, Owner::Adb(_)))
            })
            .map(|entry| entry.ticket.instance)
    }
    pub fn plan(&self) -> Plan {
        let Some(peer) = &self.selected else {
            return Plan::None;
        };
        if let Some(ticket) = self.ticket(&Owner::Peer(peer.clone())) {
            return if self.entry(ticket).expect("owned ticket").closing {
                Plan::Wait
            } else {
                Plan::Current(ticket)
            };
        }
        match self
            .target()
            .map(|slot| (slot, &self.entries[slot as usize]))
        {
            Some((slot, None)) => Plan::Ready(slot),
            Some((_, Some(entry))) if !entry.closing => Plan::Replace(entry.ticket),
            _ => Plan::Wait,
        }
    }
    /// Whether legacy discovery may choose this slot, including an existing
    /// legacy assignment. A selected native target takes priority immediately.
    pub fn adb_allowed(&self, instance: u32) -> bool {
        let Some(entry) = self.entries.get(instance as usize) else {
            return false;
        };
        if entry
            .as_ref()
            .is_some_and(|entry| matches!(entry.owner, Owner::Peer(_)))
        {
            return false;
        }
        let target = self.selected.as_ref().and_then(|peer| {
            self.ticket(&Owner::Peer(peer.clone()))
                .map(|ticket| ticket.instance)
                .or_else(|| self.target())
        });
        target != Some(instance)
    }
    pub fn adb_available(&self, instance: u32) -> bool {
        self.entries
            .get(instance as usize)
            .is_some_and(Option::is_none)
            && self.adb_allowed(instance)
    }
    pub fn next_adb_extra(&self) -> Option<u32> {
        (1..self.entries.len() as u32).find(|slot| self.adb_available(*slot))
    }
    fn allowed(&self, owner: &Owner, instance: u32) -> bool {
        if !self.enabled(owner) {
            return false;
        }
        match owner {
            Owner::Adb(serial) => {
                !serial.is_empty()
                    && serial.len() <= 1024
                    && !serial.chars().any(char::is_control)
                    && self.adb_available(instance)
            }
            Owner::Peer(peer) => {
                self.selected.as_ref() == Some(peer) && self.plan() == Plan::Ready(instance)
            }
        }
    }
    pub fn reserve(&mut self, owner: Owner, instance: u32) -> Result<Ticket> {
        ensure!(self.allowed(&owner, instance), "display slot unavailable");
        ensure!(
            self.ticket(&owner).is_none(),
            "display owner already assigned"
        );
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("display revision exhausted"))?;
        let ticket = Ticket { instance, revision };
        self.entries[instance as usize] = Some(Entry {
            owner,
            ticket,
            closing: false,
        });
        self.revision = revision;
        Ok(ticket)
    }
    pub fn close(&mut self, ticket: Ticket) -> Result<()> {
        ensure!(self.entry(ticket).is_some(), "stale display ticket");
        self.entries[ticket.instance as usize]
            .as_mut()
            .expect("validated ticket")
            .closing = true;
        Ok(())
    }
    /// Call only after actual cleanup, including unpublished startup failure.
    /// Logical stop, timeout, peer disappearance and intent changes are not proof.
    pub fn retired(&mut self, ticket: Ticket) -> Result<()> {
        ensure!(
            self.entry(ticket).is_some_and(|entry| entry.closing),
            "display not retiring"
        );
        self.entries[ticket.instance as usize] = None;
        Ok(())
    }
}
impl Reservations for Slots {
    fn open_peer(&self, ticket: Ticket, peer: &PeerId) -> bool {
        self.owner(ticket) == Some(&Owner::Peer(peer.clone()))
            && self.closing(ticket) == Some(false)
    }
}

#[cfg(test)]
mod control_tests;
#[cfg(test)]
mod tests;
