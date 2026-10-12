//! Project ordered observations into a coalescing-safe publication contract.
use super::*;

pub struct Feed {
    policy: Policy,
    display_stop: Stamp,
    auxiliary_stop: Stamp,
    source_lost: bool,
}
impl Feed {
    pub fn new(epoch: u64, capabilities: Capabilities, settings: Settings) -> Result<Self> {
        let policy = Policy::new(epoch, capabilities, settings)?;
        let initial = Stamp { epoch, revision: 0 };
        Ok(Self {
            policy,
            display_stop: initial,
            auxiliary_stop: initial,
            source_lost: false,
        })
    }
    pub fn observe(&mut self, event: Event) -> Result<()> {
        ensure!(!self.source_lost, "original activity source lost");
        self.policy.observe(event)?;
        self.remember_stops();
        Ok(())
    }
    /// Terminal for this original source. Unsupported capabilities still
    /// preserve existing behavior; supported fields become conservatively unknown.
    pub fn lost(&mut self) {
        self.source_lost = true;
        self.policy.lost();
        self.remember_stops();
    }
    fn remember_stops(&mut self) {
        let demand = self.policy.demand(true, false);
        if !demand.display {
            self.display_stop = demand.stamp;
        }
        if !demand.auxiliary {
            self.auxiliary_stop = demand.stamp;
        }
    }
    /// Viewer presence and unchanged content belong to the individual consumer.
    /// Host stop fences are recorded while each source observation is consumed.
    pub fn snapshot(&self, viewer: bool, unchanged: bool) -> ActivitySnapshot {
        ActivitySnapshot {
            demand: self.policy.demand(viewer, unchanged),
            display_stop: self.display_stop,
            auxiliary_stop: self.auxiliary_stop,
            source_lost: self.source_lost,
        }
    }
}
