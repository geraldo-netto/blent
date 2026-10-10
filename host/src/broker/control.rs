//! T853 single-owner control policy. No device, filesystem or media I/O runs here.
//! The authority worker owns Trust; its one outstanding job moves the exact
//! Pairing out and returns it. Cancellation remains immediate while storage runs.
use anyhow::{ensure, Result};
use blent_config::{
    connection::{
        control::{Command, Notice, State},
        pairing::{Lease, Pairing, PeerId, Ticket},
        wire::MAX_COUNTER,
    },
    retirement::Completion,
};
use std::collections::{BTreeMap, BTreeSet};
mod authority;
mod commands;
mod lifecycle;
pub use authority::{Admission, AuthorityJob, AuthorityKind, AuthorityOperation, AuthorityReply};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub devices: usize,
    pub links: usize,
    pub clients: usize,
    pub setup_ms: i64,
    pub approval_ms: i64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            devices: 64,
            links: 4,
            clients: 16,
            setup_ms: 60_000,
            approval_ms: 120_000,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<()> {
        ensure!((1..=512).contains(&self.devices), "invalid device capacity");
        ensure!((1..=64).contains(&self.links), "invalid link capacity");
        ensure!(
            (1..=1024).contains(&self.clients),
            "invalid client capacity"
        );
        ensure!(
            (1..=60_000).contains(&self.setup_ms),
            "invalid setup timeout"
        );
        ensure!(
            (1..=600_000).contains(&self.approval_ms),
            "invalid approval timeout"
        );
        Ok(())
    }
}

/// key is backend physical location, retained across inventory refreshes. The
/// payload remains opaque; descriptor hints never grant authentication authority.
pub struct Device<K, T> {
    pub key: K,
    pub payload: T,
    pub vendor: u16,
    pub product: u16,
    pub accessory: bool,
}
pub enum Action<T> {
    None,
    Snapshot(Vec<Notice>),
    Start { connection: u64, target: T },
    Authenticate { connection: u64, ticket: Ticket },
    Retire(Vec<u64>),
}
struct Link<K> {
    key: K,
    pairing: Option<Pairing>,
    ticket: Option<Ticket>,
    lease: Option<Lease>,
    state: State,
    trusted: bool,
    deadline: i64,
    completion: Option<Completion>,
    finished: bool,
}
pub struct Controller<K, T> {
    local: PeerId,
    limits: Limits,
    revision: u64,
    next: u64,
    devices: BTreeMap<u64, Device<K, T>>,
    links: BTreeMap<u64, Link<K>>,
    clients: BTreeSet<u64>,
    saved: Vec<PeerId>,
    healthy: bool,
    pending: Option<AuthorityOperation>,
    revokes: BTreeMap<String, PeerId>,
}
impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    /// saved must originate from the healthy, exclusively owned Trust store.
    /// Keep that store and all subsequent mutations in the authority worker.
    pub fn new(local: PeerId, mut saved: Vec<PeerId>, limits: Limits) -> Result<Self> {
        limits.validate()?;
        ensure!(saved.len() <= 1024, "invalid saved peer capacity");
        saved.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        ensure!(
            saved.windows(2).all(|pair| pair[0] != pair[1]),
            "duplicate saved peer"
        );
        Ok(Self {
            local,
            limits,
            revision: 1,
            next: 1,
            devices: BTreeMap::new(),
            links: BTreeMap::new(),
            clients: BTreeSet::new(),
            saved,
            healthy: true,
            pending: None,
            revokes: BTreeMap::new(),
        })
    }
    fn fresh(&mut self) -> Result<u64> {
        ensure!(self.next <= MAX_COUNTER, "broker identifiers exhausted");
        let id = self.next;
        self.next += 1;
        Ok(id)
    }
    fn changed(&mut self) -> Result<()> {
        ensure!(self.revision < MAX_COUNTER, "broker revision exhausted");
        self.revision += 1;
        Ok(())
    }
    fn editable(&self) -> Result<()> {
        ensure!(self.revision < MAX_COUNTER, "broker revision exhausted");
        Ok(())
    }
    fn admitting(&self) -> Result<()> {
        ensure!(
            self.healthy && self.pending.is_none() && self.revokes.is_empty(),
            "trust authority unavailable"
        );
        Ok(())
    }
    pub fn admit_client(&mut self) -> Result<u64> {
        ensure!(
            self.clients.len() < self.limits.clients,
            "client capacity reached"
        );
        let id = self.fresh()?;
        self.clients.insert(id);
        Ok(id)
    }
    /// UI lifetime does not own shared links. Runtime separately releases this
    /// client's registrations and bounded outgoing IPC queue.
    pub fn drop_client(&mut self, client: u64) -> bool {
        self.clients.remove(&client)
    }
    pub fn replace_devices(&mut self, devices: Vec<Device<K, T>>) -> Result<()> {
        self.editable()?;
        ensure!(
            devices.len() <= self.limits.devices,
            "device capacity reached"
        );
        let keys: BTreeSet<_> = devices.iter().map(|device| &device.key).collect();
        ensure!(keys.len() == devices.len(), "duplicate physical device");
        ensure!(
            self.next <= MAX_COUNTER - devices.len() as u64 + 1,
            "broker identifiers exhausted"
        );
        let mut replacement = BTreeMap::new();
        for device in devices {
            replacement.insert(self.fresh()?, device);
        }
        self.devices = replacement;
        self.changed()
    }
    pub fn snapshot(&self) -> Vec<Notice> {
        let revision = self.revision;
        let mut notices = vec![Notice::Begin {
            revision,
            local: self.local.clone(),
        }];
        notices.extend(self.devices.iter().map(|(&target, device)| Notice::Device {
            revision,
            target,
            vendor: device.vendor,
            product: device.product,
            accessory: device.accessory,
        }));
        notices.extend(self.links.iter().map(|(&connection, link)| Notice::Link {
            revision,
            connection,
            ticket: link.ticket.clone(),
            state: link.state,
            trusted: link.trusted,
        }));
        notices.extend(
            self.saved
                .iter()
                .cloned()
                .map(|peer| Notice::Saved { revision, peer }),
        );
        notices.push(Notice::End { revision });
        notices
    }
    pub fn pairing(&self, connection: u64) -> Result<&Pairing> {
        self.links
            .get(&connection)
            .and_then(|link| link.pairing.as_ref())
            .ok_or_else(|| anyhow::anyhow!("pairing unavailable"))
    }
    pub fn lease(&self, connection: u64) -> Option<&Lease> {
        self.links
            .get(&connection)
            .and_then(|link| link.lease.as_ref())
    }
    /// Runtime cancels these workers after every command/authority completion,
    /// including errors. A durable storage failure can close unrelated links.
    pub fn closing(&self) -> Vec<u64> {
        self.links
            .iter()
            .filter_map(|(&id, link)| {
                matches!(link.state, State::Closing | State::Quarantined).then_some(id)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
