use super::*;

impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    pub fn command(&mut self, client: u64, command: Command, now: i64) -> Result<Action<T>> {
        ensure!(self.clients.contains(&client), "unknown client");
        ensure!(now >= 0, "invalid monotonic time");
        command.encode()?;
        match command {
            Command::Inspect => Ok(Action::Snapshot(self.snapshot())),
            Command::Connect { revision, target } => self.connect(revision, target, now),
            Command::Decide {
                connection,
                ticket,
                allow,
                remember,
            } => self.decide(connection, ticket, allow, remember, now),
            Command::Disconnect { connection, ticket } => self.disconnect(connection, ticket),
            Command::Revoke { peer } => self.revoke(peer),
        }
    }
    fn connect(&mut self, revision: u64, target: u64, now: i64) -> Result<Action<T>> {
        self.editable()?;
        self.admitting()?;
        ensure!(revision == self.revision, "stale device snapshot");
        ensure!(
            self.links.len() < self.limits.links,
            "link capacity reached"
        );
        let device = self
            .devices
            .get(&target)
            .ok_or_else(|| anyhow::anyhow!("unknown device"))?;
        ensure!(
            !self.links.values().any(|link| link.key == device.key),
            "physical device still owned"
        );
        let deadline = now
            .checked_add(self.limits.setup_ms)
            .ok_or_else(|| anyhow::anyhow!("deadline overflow"))?;
        let key = device.key.clone();
        let payload = device.payload.clone();
        let connection = self.fresh()?;
        self.links.insert(
            connection,
            Link {
                key,
                pairing: Some(Pairing::default()),
                ticket: None,
                lease: None,
                state: State::Discovering,
                trusted: false,
                deadline,
                completion: None,
                finished: false,
            },
        );
        self.changed()?;
        Ok(Action::Start {
            connection,
            target: payload,
        })
    }
    fn decide(
        &mut self,
        connection: u64,
        ticket: Ticket,
        allow: bool,
        remember: bool,
        now: i64,
    ) -> Result<Action<T>> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            link.ticket.as_ref() == Some(&ticket) && link.state == State::Pending,
            "stale approval"
        );
        if !allow {
            close(link);
            self.changed()?;
            return Ok(Action::Retire(vec![connection]));
        }
        link.pairing
            .as_mut()
            .unwrap()
            .approve(&ticket, remember, now)?;
        link.state = State::Approved;
        self.changed()?;
        Ok(Action::Authenticate { connection, ticket })
    }
    fn disconnect(&mut self, connection: u64, ticket: Option<Ticket>) -> Result<Action<T>> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(link.ticket == ticket, "stale disconnect");
        close(link);
        self.changed()?;
        Ok(Action::Retire(vec![connection]))
    }
    fn revoke(&mut self, peer: PeerId) -> Result<Action<T>> {
        self.editable()?;
        ensure!(
            self.revokes.contains_key(peer.as_str()) || self.revokes.len() < 1024,
            "revoke capacity reached"
        );
        let mut retired = Vec::new();
        for (&id, link) in &mut self.links {
            if link
                .ticket
                .as_ref()
                .is_some_and(|ticket| ticket.peer == peer)
            {
                close(link);
                retired.push(id);
            }
        }
        self.revokes.insert(peer.as_str().to_owned(), peer);
        self.changed()?;
        Ok(Action::Retire(retired))
    }
    /// Called after bounded public Hello, never based on USB descriptor hints.
    /// A busy authority lane leaves discovery pending; caller may retry with
    /// the same bounded candidate while its setup deadline remains valid.
    pub fn identified(&mut self, connection: u64, peer: PeerId, now: i64) -> Result<Action<T>> {
        self.editable()?;
        self.admitting()?;
        let trusted = self
            .saved
            .binary_search_by(|saved| saved.as_str().cmp(peer.as_str()))
            .is_ok();
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            link.state == State::Discovering && now >= 0 && now < link.deadline,
            "stale discovery"
        );
        ensure!(link.completion.is_some(), "native owner not attached");
        let deadline = now
            .checked_add(self.limits.approval_ms)
            .ok_or_else(|| anyhow::anyhow!("deadline overflow"))?;
        let ticket =
            link.pairing
                .as_mut()
                .unwrap()
                .begin(peer, trusted, now, self.limits.approval_ms)?;
        link.deadline = deadline;
        link.ticket = Some(ticket.clone());
        link.trusted = trusted;
        link.state = if trusted {
            State::Approved
        } else {
            State::Pending
        };
        self.changed()?;
        Ok(if trusted {
            Action::Authenticate { connection, ticket }
        } else {
            Action::None
        })
    }
}

pub(super) fn close<K>(link: &mut Link<K>) {
    if let (Some(pairing), Some(ticket)) = (&mut link.pairing, &link.ticket) {
        pairing.cancel(ticket);
    }
    link.lease = None;
    link.trusted = false;
    if link.state != State::Quarantined {
        link.state = State::Closing;
    }
}
