//! Desired work observation only; never native acquisition/retirement authority.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkObservation {
    target: ActivityTarget,
    stamp: Option<Stamp>,
}
impl WorkObservation {
    /// None publication explicitly denotes a composition without activity support.
    pub fn begin(
        target: ActivityTarget,
        publication: Option<Publication>,
        source_open: bool,
    ) -> Option<Self> {
        let stamp = match publication {
            None => None,
            Some(Publication::Observed {
                capabilities,
                snapshot,
            }) => observed_stamp(target, capabilities, snapshot, source_open)?,
            _ => return None,
        };
        Some(Self { target, stamp })
    }

    /// This retains desired-work identity, not permission to acquire resources.
    pub fn permits(
        self,
        target: ActivityTarget,
        publication: Option<Publication>,
        source_open: bool,
    ) -> bool {
        if target != self.target {
            return false;
        }
        let Some(current) = Self::begin(target, publication, source_open) else {
            return false;
        };
        match (self.stamp, current.stamp, publication) {
            (None, None, _) => true,
            (Some(started), Some(_), Some(Publication::Observed { snapshot, .. })) => {
                !snapshot.must_retire(target, started)
            }
            _ => false,
        }
    }
}

fn observed_stamp(
    target: ActivityTarget,
    capabilities: Capabilities,
    snapshot: ActivitySnapshot,
    source_open: bool,
) -> Option<Option<Stamp>> {
    if snapshot.validate().is_err() || !wanted(target, snapshot) {
        return None;
    }
    if !supported(target, capabilities) {
        return Some(None);
    }
    (source_open && !snapshot.source_lost).then_some(Some(snapshot.demand.stamp))
}
fn wanted(target: ActivityTarget, snapshot: ActivitySnapshot) -> bool {
    match target {
        ActivityTarget::Display => snapshot.demand.display,
        ActivityTarget::Auxiliary => snapshot.demand.auxiliary,
    }
}
fn supported(target: ActivityTarget, capabilities: Capabilities) -> bool {
    match target {
        ActivityTarget::Display => capabilities.power || capabilities.lock || capabilities.display,
        ActivityTarget::Auxiliary => capabilities.power,
    }
}

#[cfg(test)]
mod tests;
