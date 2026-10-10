use super::*;

#[derive(Clone, Copy)]
pub struct Limits {
    pub devices: usize,
    pub links: usize,
    pub saved: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            devices: 512,
            links: 64,
            saved: 1024,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<()> {
        ensure!(
            (1..=512).contains(&self.devices),
            "invalid displayed device capacity"
        );
        ensure!(
            (1..=64).contains(&self.links),
            "invalid displayed link capacity"
        );
        ensure!(
            (1..=1024).contains(&self.saved),
            "invalid displayed saved-peer capacity"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub vendor: u16,
    pub product: u16,
    pub accessory: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub ticket: Option<Ticket>,
    pub state: State,
    pub trusted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    revision: u64,
    computer: PeerId,
    devices: BTreeMap<u64, Device>,
    links: BTreeMap<u64, Link>,
    saved: BTreeMap<String, PeerId>,
}
impl Snapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn computer(&self) -> &PeerId {
        &self.computer
    }
    pub fn devices(&self) -> &BTreeMap<u64, Device> {
        &self.devices
    }
    pub fn links(&self) -> &BTreeMap<u64, Link> {
        &self.links
    }
    pub fn saved(&self) -> &BTreeMap<String, PeerId> {
        &self.saved
    }
    fn record(&mut self, notice: Notice, limits: Limits) -> Result<bool> {
        match notice {
            Notice::Device {
                target,
                vendor,
                product,
                accessory,
                ..
            } => {
                ensure!(
                    self.devices.len() < limits.devices,
                    "device snapshot capacity exceeded"
                );
                ensure!(
                    self.devices
                        .insert(
                            target,
                            Device {
                                vendor,
                                product,
                                accessory
                            }
                        )
                        .is_none(),
                    "duplicate device"
                );
            }
            Notice::Link {
                connection,
                ticket,
                state,
                trusted,
                ..
            } => {
                self.link(connection, ticket, state, trusted, limits.links)?;
            }
            Notice::Saved { peer, .. } => {
                ensure!(
                    self.saved.len() < limits.saved,
                    "saved snapshot capacity exceeded"
                );
                ensure!(
                    self.saved.insert(peer.as_str().to_owned(), peer).is_none(),
                    "duplicate saved tablet"
                );
            }
            Notice::End { .. } => return Ok(true),
            Notice::Begin { .. } => anyhow::bail!("snapshot already started"),
        }
        Ok(false)
    }
    fn link(
        &mut self,
        connection: u64,
        ticket: Option<Ticket>,
        state: State,
        trusted: bool,
        capacity: usize,
    ) -> Result<()> {
        ensure!(
            self.links.len() < capacity,
            "link snapshot capacity exceeded"
        );
        let needs_peer = matches!(
            state,
            State::Pending | State::Approved | State::Authenticated
        ) || trusted;
        ensure!(
            !needs_peer || ticket.is_some(),
            "identified connection has no tablet ticket"
        );
        ensure!(
            self.links
                .insert(
                    connection,
                    Link {
                        ticket,
                        state,
                        trusted
                    }
                )
                .is_none(),
            "duplicate connection"
        );
        Ok(())
    }
}

/// One bounded in-progress snapshot. Invalid input drops partial state; only a
/// fresh Begin can recover. The native adapter separately bounds transaction time.
pub struct Assembler {
    limits: Limits,
    pending: Option<Snapshot>,
}
impl Assembler {
    pub fn new(limits: Limits) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            limits,
            pending: None,
        })
    }
    pub fn push(&mut self, notice: Notice) -> Result<Option<Snapshot>> {
        let previous = self.pending.take();
        // Reuse the independent flat schema's counter and field validation.
        notice.encode()?;
        if let Notice::Begin { revision, local } = notice {
            ensure!(previous.is_none(), "nested snapshot begin");
            self.pending = Some(Snapshot {
                revision,
                computer: local,
                devices: BTreeMap::new(),
                links: BTreeMap::new(),
                saved: BTreeMap::new(),
            });
            return Ok(None);
        }
        let mut snapshot = previous.ok_or_else(|| anyhow::anyhow!("snapshot begin missing"))?;
        ensure!(
            revision(&notice) == snapshot.revision,
            "mixed snapshot revisions"
        );
        if snapshot.record(notice, self.limits)? {
            return Ok(Some(snapshot));
        }
        self.pending = Some(snapshot);
        Ok(None)
    }
}
fn revision(notice: &Notice) -> u64 {
    match notice {
        Notice::Begin { revision, .. }
        | Notice::Device { revision, .. }
        | Notice::Link { revision, .. }
        | Notice::Saved { revision, .. }
        | Notice::End { revision } => *revision,
    }
}
