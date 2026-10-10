use super::*;
#[derive(Clone)]
pub(super) enum Effect {
    None,
    Commit(Lease),
    Revoke(Lease),
}
pub(super) struct Publication {
    pub bytes: Vec<u8>,
    pub remaining: usize,
    pub effect: Effect,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub(super) fn room(&self) -> Result<()> {
        ensure!(
            self.publications.len() + usize::from(self.active.is_some()) < 16,
            "management output full"
        );
        ensure!(self.events.len() < 28, "status consumer stalled");
        Ok(())
    }
    pub(super) fn queue(&mut self, message: Message, effect: Effect) -> Result<()> {
        self.room()?;
        let mut storage = [0; blent_config::connection::management::MAX_RECORD];
        let length = message.encode(&mut storage)?;
        let bytes = storage[..length].to_vec();
        self.publications.push_back(Publication {
            remaining: bytes.len(),
            bytes,
            effect,
        });
        Ok(())
    }
    pub(super) fn publish(&mut self) -> Result<()> {
        if self.active.is_some() {
            return Ok(());
        }
        if let Some(mut publication) = self.publications.pop_front() {
            let bytes = std::mem::take(&mut publication.bytes);
            self.transport()?
                .multiplexer()
                .offer(Channel::Management, bytes)
                .map_err(|_| anyhow::anyhow!("management queue ownership violated"))?;
            self.active = Some(publication);
        }
        Ok(())
    }
    pub(super) fn written(&mut self, header: Header, now: i64) -> Result<()> {
        if header.channel != Channel::Management || header.kind != Kind::Data {
            return Ok(());
        }
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("unowned management write"))?;
        active.remaining = active
            .remaining
            .checked_sub(header.length as usize)
            .ok_or_else(|| anyhow::anyhow!("management completion overflow"))?;
        if active.remaining == 0 {
            let publication = self.active.take().expect("publication owned");
            self.published(publication.effect, now)?;
        }
        Ok(())
    }
    fn published(&mut self, effect: Effect, now: i64) -> Result<()> {
        let lease = match effect {
            Effect::None => return Ok(()),
            Effect::Commit(lease) => {
                self.install(&lease, now)?;
                lease
            }
            Effect::Revoke(lease) => {
                self.resources.revoke_written(&lease)?;
                lease
            }
        };
        if let Some(message) = self.binding(&lease)?.deferred.take() {
            self.message(message, now)?;
        }
        Ok(())
    }
    fn install(&mut self, lease: &Lease, now: i64) -> Result<()> {
        let install = self.resources.commit(lease, now)?;
        self.binding(lease)?.installed = true;
        self.channels.install(lease.key.resource);
        for channel in channels::members(lease.key.resource) {
            let transport = self.transport()?;
            transport.hold(*channel, None)?;
            if install == Install::Activate {
                transport
                    .multiplexer()
                    .activate(*channel, lease.key.generation)?;
            } else {
                transport.multiplexer().close(*channel, Kind::Close)?;
            }
        }
        if install == Install::Drain && !self.binding(lease)?.closing {
            self.cancel(lease, Reason::TimedOut)?;
        }
        if self.binding(lease)?.native_retired {
            self.retire_consumers(lease)?;
        }
        Ok(())
    }
    pub(super) fn management_frame(
        &mut self,
        header: Header,
        deliver: bool,
        now: i64,
    ) -> Result<()> {
        ensure!(
            !matches!(header.kind, Kind::Close | Kind::Reset),
            "peer closed management"
        );
        if !deliver {
            return Ok(());
        }
        let mut offset = 0;
        while offset < header.length as usize {
            let (count, message) = self
                .decoder
                .feed(&self.transport.as_ref().expect("online").payload()[offset..])?;
            ensure!(count > 0, "management decoder stalled");
            offset += count;
            if let Some(message) = message {
                self.message(message, now)?;
            }
        }
        self.transport()?
            .multiplexer()
            .release(Channel::Management, header.length)
    }
    fn lease_for(&self, key: Key) -> Result<Lease> {
        let binding = self.slots[key.resource as usize - 1]
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("unregistered management resource"))?;
        ensure!(binding.lease.key == key, "stale management key");
        Ok(binding.lease.clone())
    }
    fn blocked(&self, lease: &Lease, outcome: Outcome) -> bool {
        self.active
            .iter()
            .chain(self.publications.iter())
            .any(|publication| match &publication.effect {
                Effect::Commit(pending) => pending == lease && outcome == Outcome::Admitted,
                Effect::Revoke(pending) => pending == lease && outcome == Outcome::Revoked,
                Effect::None => false,
            })
    }
    fn defer(&mut self, lease: &Lease, outcome: Outcome) -> Result<bool> {
        if !self.blocked(lease, outcome) {
            return Ok(false);
        }
        let binding = self.binding(lease)?;
        ensure!(
            binding.deferred.is_none(),
            "duplicate early management result"
        );
        binding.deferred = Some(Message::Result(lease.key, outcome));
        Ok(true)
    }
    fn message(&mut self, message: Message, now: i64) -> Result<()> {
        match message {
            Message::Capabilities(mask) => {
                ensure!(self.capabilities.is_none(), "duplicate capabilities");
                self.capabilities = Some(mask);
            }
            Message::Prepared(key) => {
                let lease = self.lease_for(key)?;
                self.prepared(&lease, now)?;
            }
            Message::Result(key, outcome) => {
                let lease = self.lease_for(key)?;
                if !self.defer(&lease, outcome)? {
                    self.outcome(&lease, outcome, now)?;
                }
            }
            Message::Revoke(key, reason) => {
                let lease = self.lease_for(key)?;
                self.resources.peer_revoke(&lease)?;
                self.cancel(&lease, reason)?;
            }
            _ => anyhow::bail!("unexpected tablet management command"),
        }
        Ok(())
    }
    fn prepared(&mut self, lease: &Lease, now: i64) -> Result<()> {
        self.room()?;
        if !self.resources.prepared(lease, now)? {
            return self.cancel(lease, Reason::TimedOut);
        }
        self.resources.start_commit(lease, now)?;
        for channel in channels::members(lease.key.resource) {
            self.transport()?
                .hold(*channel, Some(lease.key.generation))?;
        }
        self.queue(Message::Commit(lease.key), Effect::Commit(lease.clone()))
    }
    fn readable(&mut self, lease: &Lease) {
        for channel in channels::members(lease.key.resource) {
            self.mark_readable(lease.key, *channel);
        }
    }
    fn outcome(&mut self, lease: &Lease, outcome: Outcome, now: i64) -> Result<()> {
        match outcome {
            Outcome::PermissionRequired => {
                if self.resources.permission_required(lease, now)? {
                    self.notice(lease, State::PermissionRequired);
                } else {
                    self.cancel(lease, Reason::TimedOut)?;
                }
            }
            Outcome::Admitted => {
                if self.resources.admitted(lease, now)? {
                    self.notice(lease, State::Admitted);
                    self.readable(lease);
                } else {
                    self.cancel(lease, Reason::TimedOut)?;
                }
            }
            Outcome::Revoked => {
                self.resources.acknowledge(lease)?;
                self.binding(lease)?.acknowledged = true;
            }
            rejection => {
                self.resources.reject(lease, rejection)?;
                self.cancel(lease, Reason::WorkerFailed)?;
            }
        }
        Ok(())
    }
}
