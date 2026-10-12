//! Independent publication contract. A coalesced resume retains every stop fence.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityTarget {
    Display,
    Auxiliary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivitySnapshot {
    pub demand: Demand,
    pub display_stop: Stamp,
    pub auxiliary_stop: Stamp,
    pub source_lost: bool,
}
impl ActivitySnapshot {
    pub fn validate(self) -> Result<()> {
        ensure!(self.demand.stamp.epoch > 0, "invalid activity source epoch");
        ensure!(
            self.contains(self.display_stop) && self.contains(self.auxiliary_stop),
            "foreign or future activity stop fence"
        );
        let auxiliary = !matches!(
            self.demand.reason,
            Some(Reason::PowerUnknown | Reason::Suspending | Reason::Unavailable)
        );
        ensure!(
            self.demand.display == self.demand.reason.is_none()
                && self.demand.auxiliary == auxiliary,
            "inconsistent activity demand"
        );
        Ok(())
    }
    fn contains(self, stamp: Stamp) -> bool {
        stamp.epoch == self.demand.stamp.epoch && stamp.revision <= self.demand.stamp.revision
    }
    /// Original work must stop even if the latest desired state has resumed.
    /// This is never acquisition permission or native retirement evidence.
    pub fn must_retire(self, target: ActivityTarget, started: Stamp) -> bool {
        if self.validate().is_err() || !self.contains(started) {
            return true;
        }
        let (wanted, stopped) = match target {
            ActivityTarget::Display => (self.demand.display, self.display_stop),
            ActivityTarget::Auxiliary => (self.demand.auxiliary, self.auxiliary_stop),
        };
        !wanted || stopped.revision > started.revision
    }
}
