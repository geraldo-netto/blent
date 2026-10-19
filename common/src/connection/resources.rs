//! T851: fixed-capacity resource authority, independent of native ownership.
//!
//! The authenticated connection owner serializes these calls. Reserve bounded
//! management output before committing an outgoing transition; preserve FIFO
//! through cancellation. Failed/partial writes disconnect, never roll back.
//! `commit` and `revoke_written` are whole-record write-completion barriers on
//! the host. Queue peer events until those completions have been processed.
use super::{
    management::{Key, Outcome, Resource},
    pairing::Ticket,
    wire::MAX_COUNTER,
};
use anyhow::{ensure, Result};
mod timeout;
mod transitions;
pub use timeout::AdmissionTimeout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Host,
    Tablet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Install {
    Activate,
    Drain,
}
/// Identifies the local registration as well as the authenticated connection.
/// Wire IDs alone are never accepted in asynchronous completion callbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub connection: Ticket,
    pub key: Key,
    pub owner: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Offered,
    Prepared,
    Committing,
    Committed,
    Admitted,
}
struct Slot {
    lease: Lease,
    deadline: i64,
    stage: Stage,
    closing: bool,
    permission_reported: bool,
    rejected: bool,
    peer_revoke: bool,
    revoke_written: bool,
    settled: bool,
    installed: bool,
    native_retired: bool,
    channels_retired: bool,
}
impl Slot {
    fn new(lease: Lease, deadline: i64) -> Self {
        Self {
            lease,
            deadline,
            stage: Stage::Offered,
            closing: false,
            permission_reported: false,
            rejected: false,
            peer_revoke: false,
            revoke_written: false,
            settled: false,
            installed: false,
            native_retired: false,
            channels_retired: true,
        }
    }
    fn live(&self, now: i64) -> Result<()> {
        ensure!(
            !self.closing && now >= 0 && now < self.deadline,
            "expired or revoked resource"
        );
        Ok(())
    }
}
/// Construct only from a currently authenticated Pairing lease. The adapter
/// calls disconnect before dispatching any callback after pairing/link loss.
/// A closing slot remains quarantined indefinitely if actual retirement cannot
/// be established; it cannot stall allocation of another resource's slot.
pub struct Resources {
    role: Role,
    connection: Ticket,
    connected: bool,
    request: u64,
    last_requests: [u64; 4],
    generations: [u64; 4],
    slots: [Option<Slot>; 4],
}
impl Resources {
    pub fn new(role: Role, connection: Ticket) -> Result<Self> {
        ensure!(
            (1..=MAX_COUNTER).contains(&connection.epoch),
            "invalid connection epoch"
        );
        Ok(Self {
            role,
            connection,
            connected: true,
            request: 0,
            last_requests: [0; 4],
            generations: [1; 4],
            slots: [None, None, None, None],
        })
    }
    pub fn begin(
        &mut self,
        resource: Resource,
        owner: u64,
        now: i64,
        timeout: i64,
    ) -> Result<Lease> {
        ensure!(
            self.role == Role::Host && self.request < MAX_COUNTER,
            "host request unavailable"
        );
        let key = Key {
            resource,
            request: self.request + 1,
            generation: self.generations[resource as usize - 1],
        };
        let lease = self.insert(key, owner, now, timeout)?;
        self.request = key.request;
        Ok(lease)
    }
    pub fn offer(&mut self, key: Key, owner: u64, now: i64, timeout: i64) -> Result<Lease> {
        ensure!(self.role == Role::Tablet, "only tablet accepts offers");
        self.insert(key, owner, now, timeout)
    }
    fn insert(&mut self, key: Key, owner: u64, now: i64, timeout: i64) -> Result<Lease> {
        key.validate()?;
        ensure!(
            self.connected && (1..=MAX_COUNTER).contains(&owner),
            "invalid registration"
        );
        ensure!(now >= 0, "invalid resource deadline");
        let timeout = AdmissionTimeout::new(timeout)?.milliseconds();
        let deadline = now
            .checked_add(timeout)
            .ok_or_else(|| anyhow::anyhow!("deadline overflow"))?;
        let index = key.resource as usize - 1;
        ensure!(self.slots[index].is_none(), "resource still owned");
        ensure!(
            key.request > self.last_requests[index] && key.generation == self.generations[index],
            "stale offer"
        );
        let lease = Lease {
            connection: self.connection.clone(),
            key,
            owner,
        };
        self.slots[index] = Some(Slot::new(lease.clone(), deadline));
        self.last_requests[index] = key.request;
        Ok(lease)
    }
    fn slot(&mut self, lease: &Lease) -> Result<&mut Slot> {
        let slot = self.slots[lease.key.resource as usize - 1]
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("resource not owned"))?;
        ensure!(&slot.lease == lease, "stale resource lease");
        Ok(slot)
    }
    fn online(&mut self, lease: &Lease) -> Result<&mut Slot> {
        ensure!(self.connected, "connection lost");
        let slot = self.slot(lease)?;
        ensure!(!slot.settled, "transaction settled");
        Ok(slot)
    }
    pub fn current(&self, lease: &Lease) -> bool {
        self.connected
            && self.slots[lease.key.resource as usize - 1]
                .as_ref()
                .is_some_and(|s| s.lease == *lease && !s.closing && s.stage == Stage::Admitted)
    }
    pub fn revoke(&mut self, lease: &Lease) -> Result<bool> {
        let slot = self.slot(lease)?;
        let changed = !slot.closing;
        slot.closing = true;
        Ok(changed)
    }
    pub fn expire(&mut self, now: i64) -> Result<[Option<Lease>; 4]> {
        ensure!(now >= 0, "invalid monotonic time");
        let mut expired = [None, None, None, None];
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if let Some(slot) = slot {
                if !slot.closing && slot.stage != Stage::Admitted && now >= slot.deadline {
                    slot.closing = true;
                    expired[index] = Some(slot.lease.clone());
                }
            }
        }
        Ok(expired)
    }
    /// Physical link loss waives only the peer settlement barrier. Native
    /// consumers and writers still need actual retirement, even after a kill.
    pub fn disconnect(&mut self) {
        self.connected = false;
        for slot in self.slots.iter_mut().flatten() {
            slot.closing = true;
            slot.settled = true;
        }
    }
    pub fn native_retired(&mut self, lease: &Lease) -> Result<()> {
        let slot = self.slot(lease)?;
        ensure!(
            slot.closing && !slot.native_retired,
            "native retirement not due"
        );
        slot.native_retired = true;
        Ok(())
    }
    /// Adapter proves both channel terminals plus consumer/writer retirement.
    /// After disconnect, actual consumer/writer exit replaces peer terminal
    /// evidence. Display requires BOTH its control and display channels.
    pub fn channels_retired(&mut self, lease: &Lease) -> Result<()> {
        let slot = self.slot(lease)?;
        ensure!(
            slot.closing && slot.installed && !slot.channels_retired,
            "channel retirement not due"
        );
        slot.channels_retired = true;
        Ok(())
    }
    pub fn finish(&mut self, lease: &Lease) -> Result<()> {
        let slot = self.slot(lease)?;
        ensure!(
            slot.closing && slot.settled && slot.native_retired && slot.channels_retired,
            "resource not retired"
        );
        let installed = slot.installed;
        let index = lease.key.resource as usize - 1;
        // MAX_COUNTER+1 is an exhausted sentinel; insert rejects it. A never
        // installed offer must reuse the untouched inactive mux generation.
        self.generations[index] += u64::from(installed);
        self.slots[index] = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::pairing::PeerId;
    #[test]
    fn t851_request_and_generation_exhaustion_never_wrap() {
        let mut resources = Resources::new(
            Role::Host,
            Ticket {
                epoch: 1,
                peer: PeerId::parse("ab".repeat(32)).unwrap(),
            },
        )
        .unwrap();
        resources.request = MAX_COUNTER - 1;
        resources.generations[0] = MAX_COUNTER;
        let lease = resources.begin(Resource::DisplaySession, 1, 0, 10).unwrap();
        assert_eq!(lease.key.request, MAX_COUNTER);
        assert_eq!(lease.key.generation, MAX_COUNTER);
        resources.prepared(&lease, 1).unwrap();
        resources.start_commit(&lease, 2).unwrap();
        resources.commit(&lease, 3).unwrap();
        resources.revoke(&lease).unwrap();
        resources.revoke_written(&lease).unwrap();
        resources.acknowledge(&lease).unwrap();
        resources.native_retired(&lease).unwrap();
        resources.channels_retired(&lease).unwrap();
        resources.finish(&lease).unwrap();
        assert!(resources.begin(Resource::Camera, 1, 0, 10).is_err());
        resources.request = 0;
        assert!(resources.begin(Resource::DisplaySession, 1, 0, 10).is_err());
        assert_eq!(resources.generations[0], MAX_COUNTER + 1);
    }
}
