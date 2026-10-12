use super::*;

pub struct Policy {
    stamp: Stamp,
    capabilities: Capabilities,
    settings: Settings,
    state: HostState,
}
impl Policy {
    pub fn new(epoch: u64, capabilities: Capabilities, settings: Settings) -> Result<Self> {
        ensure!(epoch > 0, "invalid host observation source epoch");
        Ok(Self {
            stamp: Stamp { epoch, revision: 0 },
            capabilities,
            settings,
            state: HostState::default(),
        })
    }
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
    /// Rejected observations leave current policy unchanged. A new backend must
    /// create a new owner/epoch, never reset an existing source's revision.
    pub fn observe(&mut self, event: Event) -> Result<()> {
        ensure!(
            event.epoch == self.stamp.epoch,
            "foreign host observation source"
        );
        ensure!(
            event.revision > self.stamp.revision,
            "stale host observation revision"
        );
        validate(event.state, self.capabilities)?;
        self.state = event.state;
        self.stamp.revision = event.revision;
        Ok(())
    }
    pub fn demand(&self, viewer: bool, unchanged: bool) -> Demand {
        let power = self.power_reason();
        let reason = power.or_else(|| self.display_reason(viewer));
        Demand {
            stamp: self.stamp,
            display: reason.is_none(),
            auxiliary: power.is_none(),
            unchanged,
            reason,
        }
    }
    fn power_reason(&self) -> Option<Reason> {
        match self.state.power {
            Some(Power::Suspending) => Some(Reason::Suspending),
            Some(Power::Unavailable) => Some(Reason::Unavailable),
            None if self.capabilities.power => Some(Reason::PowerUnknown),
            _ => None,
        }
    }
    fn display_reason(&self, viewer: bool) -> Option<Reason> {
        self.lock_reason()
            .or_else(|| self.display_off_reason())
            .or_else(|| (self.settings.pause_without_viewer && !viewer).then_some(Reason::NoViewer))
    }
    fn lock_reason(&self) -> Option<Reason> {
        if self.settings.pause_on_lock && self.capabilities.lock {
            match self.state.locked {
                Some(true) => Some(Reason::Locked),
                None => Some(Reason::LockUnknown),
                _ => None,
            }
        } else {
            None
        }
    }
    fn display_off_reason(&self) -> Option<Reason> {
        if self.settings.pause_on_display_off && self.capabilities.display {
            match self.state.display_on {
                Some(false) => Some(Reason::DisplayOff),
                None => Some(Reason::DisplayUnknown),
                _ => None,
            }
        } else {
            None
        }
    }
}
pub(super) fn validate(state: HostState, capabilities: Capabilities) -> Result<()> {
    ensure!(
        state.power.is_none() || capabilities.power,
        "power observation backend is unsupported"
    );
    ensure!(
        state.locked.is_none() || capabilities.lock,
        "lock observation backend is unsupported"
    );
    ensure!(
        state.display_on.is_none() || capabilities.display,
        "display observation backend is unsupported"
    );
    Ok(())
}
