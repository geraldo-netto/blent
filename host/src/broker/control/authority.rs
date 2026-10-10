use super::{commands::close, *};
use blent_config::connection::{
    pairing::Phase,
    trust::{Trust, TrustStorage},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityKind {
    Authenticate,
    Revoke,
}
/// Correlates worker completion and local IPC acknowledgements. The unique ID
/// is local bookkeeping, never pairing or media authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityOperation {
    pub id: u64,
    pub connection: Option<u64>,
    pub peer: PeerId,
    pub kind: AuthorityKind,
}
enum Operation {
    Authenticate {
        ticket: Ticket,
        proven: PeerId,
        now: i64,
    },
    Revoke,
}
/// Move this to the single bounded storage worker. Never run on the media lane.
pub struct AuthorityJob {
    pub(super) pending: AuthorityOperation,
    pairing: Pairing,
    operation: Operation,
}
/// Only a completed AuthorityJob can construct this return capability.
pub struct AuthorityReply {
    pub(super) pending: AuthorityOperation,
    pairing: Pairing,
    result: Result<Option<Lease>>,
    healthy: bool,
}
pub struct Admission {
    pub connection: u64,
    pub lease: Lease,
}
impl AuthorityJob {
    pub fn operation(&self) -> &AuthorityOperation {
        &self.pending
    }
    pub fn run<S: TrustStorage>(mut self, trust: &mut Trust<S>) -> AuthorityReply {
        let result = match self.operation {
            Operation::Authenticate {
                ticket,
                proven,
                now,
            } => trust
                .authenticated(&mut self.pairing, &ticket, &proven, now)
                .map(Some),
            Operation::Revoke => trust
                .revoke(&mut self.pairing, &self.pending.peer)
                .map(|_| None),
        };
        AuthorityReply {
            pending: self.pending,
            pairing: self.pairing,
            result,
            healthy: trust.healthy(),
        }
    }
}
impl AuthorityReply {
    pub fn operation(&self) -> &AuthorityOperation {
        &self.pending
    }
}
impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    /// TLS proof comes only from the verified adapter. Returned job owns the
    /// original Pairing; the link cannot publish authority until it returns.
    pub fn prepare_authentication(
        &mut self,
        connection: u64,
        ticket: &Ticket,
        proven: PeerId,
        now: i64,
    ) -> Result<AuthorityJob> {
        self.editable()?;
        self.admitting()?;
        let link = self
            .links
            .get(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            link.state == State::Approved && link.ticket.as_ref() == Some(ticket),
            "stale TLS proof"
        );
        ensure!(
            now >= 0 && now < link.deadline && proven == ticket.peer,
            "expired or changed TLS proof"
        );
        ensure!(
            link.pairing
                .as_ref()
                .is_some_and(|pairing| pairing.phase() == Phase::Approved),
            "pairing unavailable"
        );
        let pending = AuthorityOperation {
            id: self.fresh()?,
            connection: Some(connection),
            peer: ticket.peer.clone(),
            kind: AuthorityKind::Authenticate,
        };
        let pairing = self
            .links
            .get_mut(&connection)
            .unwrap()
            .pairing
            .take()
            .unwrap();
        self.pending = Some(pending.clone());
        Ok(AuthorityJob {
            pending,
            pairing,
            operation: Operation::Authenticate {
                ticket: ticket.clone(),
                proven,
                now,
            },
        })
    }
    /// Revokes are serialized AFTER a started remember. Cancellation of every
    /// matching live link already happened when Command::Revoke was accepted.
    pub fn next_authority(&mut self) -> Result<Option<AuthorityJob>> {
        ensure!(self.pending.is_none(), "authority job still owned");
        let Some(peer) = self.revokes.values().next().cloned() else {
            return Ok(None);
        };
        let connection = self.links.iter().find_map(|(&id, link)| {
            link.ticket
                .as_ref()
                .is_some_and(|ticket| ticket.peer == peer)
                .then_some(id)
        });
        let pending = AuthorityOperation {
            id: self.fresh()?,
            connection,
            peer: peer.clone(),
            kind: AuthorityKind::Revoke,
        };
        let pairing = self.revoke_pairing(connection)?;
        self.revokes.remove(peer.as_str());
        self.pending = Some(pending.clone());
        Ok(Some(AuthorityJob {
            pending,
            pairing,
            operation: Operation::Revoke,
        }))
    }
    fn revoke_pairing(&mut self, connection: Option<u64>) -> Result<Pairing> {
        match connection {
            Some(id) => self
                .links
                .get_mut(&id)
                .unwrap()
                .pairing
                .take()
                .ok_or_else(|| anyhow::anyhow!("pairing unavailable")),
            // No live attempt exists. An idle Pairing lets the existing revoke
            // policy invalidate nothing before durable deletion; never proof.
            None => Ok(Pairing::default()),
        }
    }
    pub fn complete_authority(
        &mut self,
        reply: AuthorityReply,
        now: i64,
    ) -> Result<Option<Admission>> {
        ensure!(
            self.pending.as_ref() == Some(&reply.pending),
            "stale authority reply"
        );
        ensure!(now >= 0, "invalid monotonic time");
        self.editable()?;
        self.pending = None;
        if let Some(id) = reply.pending.connection {
            let link = self.links.get_mut(&id).unwrap();
            link.pairing = Some(reply.pairing);
            if matches!(link.state, State::Closing | State::Quarantined) {
                close(link);
            }
        }
        self.healthy &= reply.healthy;
        if !self.healthy {
            self.close_all();
        }
        self.changed()?;
        match reply.result {
            Ok(Some(lease)) => self.admit_proven(reply.pending.connection.unwrap(), lease, now),
            Ok(None) => {
                self.saved.retain(|saved| saved != &reply.pending.peer);
                Ok(None)
            }
            Err(error) => {
                if let Some(id) = reply.pending.connection {
                    close(self.links.get_mut(&id).unwrap());
                }
                Err(error)
            }
        }
    }
    fn admit_proven(
        &mut self,
        connection: u64,
        lease: Lease,
        now: i64,
    ) -> Result<Option<Admission>> {
        // Saved notices describe durable writes, not current admission. A queued
        // revoke still closes links and blocks admission until deletion returns.
        self.remember_saved(&lease)?;
        let link = self.links.get_mut(&connection).unwrap();
        if link.state != State::Approved || now >= link.deadline || !self.healthy {
            close(link);
            return Ok(None);
        }
        ensure!(
            link.pairing.as_ref().unwrap().current(&lease),
            "invalid returned authority"
        );
        link.state = State::Authenticated;
        link.lease = Some(lease.clone());
        Ok(Some(Admission { connection, lease }))
    }
    fn remember_saved(&mut self, lease: &Lease) -> Result<()> {
        if !lease.remember {
            return Ok(());
        }
        if let Err(index) = self
            .saved
            .binary_search_by(|peer| peer.as_str().cmp(lease.ticket.peer.as_str()))
        {
            ensure!(self.saved.len() < 1024, "saved peer capacity reached");
            self.saved.insert(index, lease.ticket.peer.clone());
        }
        Ok(())
    }
    fn close_all(&mut self) {
        for link in self.links.values_mut() {
            close(link);
        }
    }
}
