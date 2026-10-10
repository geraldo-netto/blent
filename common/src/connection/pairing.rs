//! Approval/authority lifecycle independent of TLS, storage, UI and native I/O.
use super::wire::MAX_COUNTER;
use anyhow::{ensure, Result};

/// SHA-256 of the exact DER certificate, computed by the stock TLS adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerId(String);
impl PeerId {
    pub fn parse(value: String) -> Result<Self> {
        ensure!(
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid peer fingerprint"
        );
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub epoch: u64,
    pub peer: PeerId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub ticket: Ticket,
    pub remember: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Pending,
    Approved,
    Authenticated,
    Closing,
}

/// One owner serializes events. A new attempt cannot replace a Closing native
/// worker until the adapter reports actual retirement. Approval never grants
/// media permission. Authentication proof must come from the verified TLS
/// endpoint, never from bytes parsed from a remote management message.
pub struct Pairing {
    phase: Phase,
    epoch: u64,
    ticket: Option<Ticket>,
    deadline: i64,
    remember: bool,
}
impl Default for Pairing {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            epoch: 0,
            ticket: None,
            deadline: 0,
            remember: false,
        }
    }
}
impl Pairing {
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn begin(&mut self, peer: PeerId, trusted: bool, now: i64, timeout: i64) -> Result<Ticket> {
        ensure!(self.phase == Phase::Idle, "previous worker still owned");
        ensure!(
            now >= 0 && (1..=600_000).contains(&timeout),
            "invalid approval timeout"
        );
        let deadline = now
            .checked_add(timeout)
            .ok_or_else(|| anyhow::anyhow!("deadline overflow"))?;
        ensure!(self.epoch < MAX_COUNTER, "connection epoch exhausted");
        self.epoch += 1;
        let ticket = Ticket {
            epoch: self.epoch,
            peer,
        };
        self.ticket = Some(ticket.clone());
        self.deadline = deadline;
        self.remember = false;
        self.phase = if trusted {
            Phase::Approved
        } else {
            Phase::Pending
        };
        Ok(ticket)
    }
    fn matches(&self, ticket: &Ticket) -> bool {
        self.ticket.as_ref() == Some(ticket)
    }
    pub fn approve(&mut self, ticket: &Ticket, remember: bool, now: i64) -> Result<()> {
        ensure!(
            self.matches(ticket) && self.phase == Phase::Pending,
            "stale approval"
        );
        ensure!(now >= 0 && now < self.deadline, "approval expired");
        self.remember = remember;
        self.phase = Phase::Approved;
        Ok(())
    }
    /// The adapter supplies the pin proven by TLS. Persist trust only after this
    /// succeeds, and recheck the lease before installing any local endpoint.
    pub fn authenticated(
        &mut self,
        ticket: &Ticket,
        proven_peer: &PeerId,
        now: i64,
    ) -> Result<Lease> {
        ensure!(
            self.matches(ticket) && self.phase == Phase::Approved,
            "unapproved connection"
        );
        ensure!(
            now >= 0 && now < self.deadline && proven_peer == &ticket.peer,
            "stale or changed TLS proof"
        );
        self.phase = Phase::Authenticated;
        Ok(Lease {
            ticket: ticket.clone(),
            remember: self.remember,
        })
    }
    pub fn current(&self, lease: &Lease) -> bool {
        self.phase == Phase::Authenticated
            && self.matches(&lease.ticket)
            && lease.remember == self.remember
    }
    pub fn cancel(&mut self, ticket: &Ticket) -> bool {
        if !self.matches(ticket) || self.phase == Phase::Idle {
            return false;
        }
        self.phase = Phase::Closing;
        true
    }
    pub fn revoke(&mut self, peer: &PeerId) -> bool {
        let Some(ticket) = self.ticket.clone() else {
            return false;
        };
        if &ticket.peer != peer {
            return false;
        }
        self.cancel(&ticket)
    }
    pub fn expire(&mut self, now: i64) -> Result<bool> {
        ensure!(now >= 0, "invalid monotonic time");
        if !matches!(self.phase, Phase::Pending | Phase::Approved) || now < self.deadline {
            return Ok(false);
        }
        self.phase = Phase::Closing;
        Ok(true)
    }
    /// Call only after native worker exit/handle retirement is observed. Logical
    /// cancellation, a timeout, or sending a kill signal is not that evidence.
    pub fn retired(&mut self, ticket: &Ticket) -> Result<()> {
        ensure!(
            self.matches(ticket) && self.phase == Phase::Closing,
            "worker not retired"
        );
        self.phase = Phase::Idle;
        self.ticket = None;
        self.remember = false;
        Ok(())
    }
}
