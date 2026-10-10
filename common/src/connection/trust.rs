//! T868 persisted peer approval, independent of TLS, UI and storage adapters.
//! One serialized owner must govern both this state and every associated Pairing.
//! I/O belongs off the UI thread. Stored approval never substitutes for TLS proof.
use super::pairing::{Lease, Pairing, PeerId, Ticket};
use anyhow::{ensure, Result};
pub mod codec;

pub const DEFAULT_CAPACITY: usize = 64;
pub const MAX_PEERS: usize = 1024;

/// The adapter holds an exclusive lifetime ownership lock. Missing state is empty
/// only if no interrupted transaction exists. replace() must durably mark pending
/// work, atomically replace complete bytes, sync publication, then retire the mark.
/// Startup with pending/temp state fails closed; no automatic trust recovery.
pub trait TrustStorage {
    fn load(&mut self) -> Result<Option<Vec<u8>>>;
    fn replace(&mut self, record: &[u8]) -> Result<()>;
}

/// Canonical sorted unique peers, bound to the local certificate. Zero revision
/// is reserved for an absent store and can never appear in an encoded record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub local: PeerId,
    pub revision: u64,
    pub peers: Vec<PeerId>,
}

pub struct Trust<S: TrustStorage> {
    store: S,
    snapshot: Snapshot,
    capacity: usize,
    healthy: bool,
}
impl<S: TrustStorage> Trust<S> {
    pub fn open(mut store: S, local: PeerId, capacity: usize) -> Result<Self> {
        ensure!(
            (1..=MAX_PEERS).contains(&capacity),
            "invalid trust capacity"
        );
        let snapshot = match store.load()? {
            Some(bytes) => codec::decode(&bytes)?,
            None => Snapshot {
                local: local.clone(),
                revision: 0,
                peers: Vec::new(),
            },
        };
        ensure!(
            snapshot.local == local,
            "trust belongs to another local identity"
        );
        ensure!(
            snapshot.peers.len() <= capacity,
            "stored trust exceeds selected capacity"
        );
        Ok(Self {
            store,
            snapshot,
            capacity,
            healthy: true,
        })
    }
    pub fn healthy(&self) -> bool {
        self.healthy
    }
    /// Startup presentation only; persisted approval never replaces TLS proof.
    pub fn approved_peers(&self) -> Result<&[PeerId]> {
        ensure!(self.healthy, "trust storage unavailable");
        Ok(&self.snapshot.peers)
    }
    pub fn trusted(&self, peer: &PeerId) -> bool {
        self.healthy && self.find(peer).is_ok()
    }
    fn find(&self, peer: &PeerId) -> std::result::Result<usize, usize> {
        self.snapshot
            .peers
            .binary_search_by(|candidate| candidate.as_str().cmp(peer.as_str()))
    }
    /// T852: stored approval only selects the prompt state. Every connection
    /// still needs fresh stock TLS authentication before a lease is returned.
    pub fn begin(
        &self,
        pairing: &mut Pairing,
        peer: PeerId,
        now: i64,
        timeout: i64,
    ) -> Result<Ticket> {
        ensure!(self.healthy, "trust storage unavailable");
        let trusted = self.trusted(&peer);
        pairing.begin(peer, trusted, now, timeout)
    }
    /// Trusted adapter callback after successful stock TLS and protocol proof.
    /// If authentication or persistence fails, cancel this matching attempt;
    /// native retirement is still required before a replacement can begin.
    pub fn authenticated(
        &mut self,
        pairing: &mut Pairing,
        ticket: &Ticket,
        proven: &PeerId,
        now: i64,
    ) -> Result<Lease> {
        let result = (|| {
            ensure!(self.healthy, "trust storage unavailable");
            let lease = pairing.authenticated(ticket, proven, now)?;
            if lease.remember {
                self.remember(pairing, &lease)?;
            }
            Ok(lease)
        })();
        if result.is_err() {
            pairing.cancel(ticket);
        }
        result
    }
    /// A no-op still requires a current authenticated remembered lease. Callers
    /// serialize this complete operation with all approval/revoke events.
    pub fn remember(&mut self, pairing: &Pairing, lease: &Lease) -> Result<bool> {
        ensure!(self.healthy, "trust storage unavailable");
        ensure!(
            lease.remember && pairing.current(lease),
            "unproven or stale remembered approval"
        );
        let index = match self.find(&lease.ticket.peer) {
            Ok(_) => return Ok(false),
            Err(index) => index,
        };
        ensure!(
            self.snapshot.peers.len() < self.capacity,
            "trust capacity reached"
        );
        let mut peers = self.snapshot.peers.clone();
        peers.insert(index, lease.ticket.peer.clone());
        self.persist(peers)?;
        Ok(true)
    }
    /// Invalidate the supplied live authority BEFORE fallible storage work.
    /// A multi-link broker must revoke every Pairing for this peer in its same
    /// serialized command lane before reconnect admission resumes.
    pub fn revoke(&mut self, pairing: &mut Pairing, peer: &PeerId) -> Result<bool> {
        pairing.revoke(peer);
        ensure!(self.healthy, "trust storage unavailable");
        let Ok(index) = self.find(peer) else {
            return Ok(false);
        };
        self.snapshot.peers.remove(index);
        self.persist(self.snapshot.peers.clone())?;
        Ok(true)
    }
    fn persist(&mut self, peers: Vec<PeerId>) -> Result<()> {
        // Poison first: any serialization, counter or storage error keeps all
        // reconnect admission closed. Existing memory never restores authority.
        self.healthy = false;
        ensure!(
            self.snapshot.revision < i64::MAX as u64,
            "trust revision exhausted"
        );
        let next = Snapshot {
            local: self.snapshot.local.clone(),
            revision: self.snapshot.revision + 1,
            peers,
        };
        self.store.replace(&codec::encode(&next)?)?;
        self.snapshot = next;
        self.healthy = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
