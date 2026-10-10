use super::*;
impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub(super) fn notice(&mut self, lease: &Lease, state: State) {
        // At most four leases and a bounded publication queue. Commands reserve
        // status headroom before transition; peer duplicates are rejected.
        self.events.push_back(Event::State(lease.clone(), state));
    }
    pub fn cancel(&mut self, lease: &Lease, reason: Reason) -> Result<()> {
        if self.binding(lease)?.closing {
            return Ok(());
        }
        self.room()?;
        self.resources.revoke(lease)?;
        self.binding(lease)?.closing = true;
        if self.binding(lease)?.installed {
            for channel in channels::members(lease.key.resource) {
                let mux = self.transport()?.multiplexer();
                if mux.accepts_data(*channel, lease.key.generation) {
                    mux.close(*channel, Kind::Close)?;
                }
            }
        }
        self.queue(
            Message::Revoke(lease.key, reason),
            Effect::Revoke(lease.clone()),
        )?;
        self.notice(lease, State::Closing);
        Ok(())
    }
    pub fn tick(&mut self, now: i64) -> Result<()> {
        self.room()?;
        for lease in self.resources.expire(now)?.into_iter().flatten() {
            self.cancel(&lease, Reason::TimedOut)?;
        }
        Ok(())
    }
    /// Call after both directions of every local bridge for this resource have
    /// actually stopped. Signals/abort requests/timeouts are not this evidence.
    pub fn native_retired(&mut self, lease: &Lease) -> Result<()> {
        self.binding(lease)?;
        self.resources.native_retired(lease)?;
        self.binding(lease)?.native_retired = true;
        if self.binding(lease)?.installed {
            self.retire_consumers(lease)?;
        }
        self.settle()
    }
    pub(super) fn retire_consumers(&mut self, lease: &Lease) -> Result<()> {
        self.channels.retire(lease.key.resource);
        if let Some(transport) = self.transport.as_mut() {
            for channel in channels::members(lease.key.resource) {
                transport
                    .multiplexer()
                    .consumer_retired(*channel, lease.key.generation)?;
            }
        }
        Ok(())
    }
    pub fn disconnect(&mut self) -> Result<()> {
        // Drop framed I/O before waiving peer terminals. No borrowed writer can
        // survive a poll return; the separate native Owner still needs retirement.
        self.transport.take();
        self.resources.disconnect();
        self.publications.clear();
        self.active = None;
        let leases = self
            .slots
            .iter_mut()
            .flatten()
            .filter_map(|binding| {
                let changed = !binding.closing;
                binding.closing = true;
                binding.acknowledged = true;
                binding.deferred = None;
                changed.then(|| binding.lease.clone())
            })
            .collect::<Vec<_>>();
        for lease in leases {
            self.notice(&lease, State::Closing);
        }
        self.settle()
    }
    pub(super) fn settle(&mut self) -> Result<()> {
        for index in 0..self.slots.len() {
            let ready = self.slots[index].as_ref().is_some_and(|binding| {
                binding.closing && binding.native_retired && binding.acknowledged
            });
            if ready {
                let lease = self.slots[index]
                    .as_ref()
                    .expect("ready binding")
                    .lease
                    .clone();
                self.settle_one(&lease)?;
            }
        }
        Ok(())
    }
    fn settle_one(&mut self, lease: &Lease) -> Result<()> {
        let binding = self.binding(lease)?;
        if !binding.closing || !binding.native_retired || !binding.acknowledged {
            return Ok(());
        }
        if binding.installed && !self.finish_channels(lease)? {
            return Ok(());
        }
        self.resources.finish(lease)?;
        self.slots[lease.key.resource as usize - 1] = None;
        self.notice(lease, State::Retired);
        Ok(())
    }
    fn finish_channels(&mut self, lease: &Lease) -> Result<bool> {
        if self.binding(lease)?.channels_retired {
            return Ok(true);
        }
        if let Some(transport) = self.transport.as_mut() {
            let mux = transport.multiplexer();
            if !channels::members(lease.key.resource)
                .iter()
                .all(|channel| mux.retired(*channel))
            {
                return Ok(false);
            }
            if lease.key.generation < MAX_COUNTER {
                for channel in channels::members(lease.key.resource) {
                    mux.replace_channel(*channel, lease.key.generation + 1)?;
                }
            }
        }
        self.resources.channels_retired(lease)?;
        self.binding(lease)?.channels_retired = true;
        Ok(true)
    }
}
