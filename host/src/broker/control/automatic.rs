//! Bounded scheduling hints for remembered-peer discovery. Physical locations
//! select attempts only; the existing pairing/TLS flow remains authoritative.
use super::*;
use blent_config::connection::control::Availability;

/// Bounded per-device scheduling state. A confirmed detach drops this record;
/// an unavailable inventory does not establish detach.
#[derive(Default)]
pub(super) struct Retry {
    deadline: i64,
    delay: i64,
    suppressed: bool,
}
impl Retry {
    pub(super) fn attempted(&mut self, now: i64) {
        self.delay = self.delay.saturating_mul(2).clamp(2_000, 60_000);
        self.deadline = now.saturating_add(self.delay);
        self.suppressed = false;
    }
    pub(super) fn suppress(&mut self) {
        self.suppressed = true;
    }
    fn ready(&self, now: i64) -> bool {
        !self.suppressed && now >= self.deadline
    }
}

impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    /// One action per call; live and retiring owners both reserve their key.
    /// Called only by the running broker, never during shutdown draining.
    pub fn automatic(&mut self, now: i64) -> Result<Action<T>> {
        ensure!(now >= 0, "invalid monotonic time");
        if !self.automatic_admitted() {
            return Ok(Action::None);
        }
        let target = self
            .devices
            .iter()
            .find_map(|(&id, device)| self.automatic_ready(device, now).then_some(id));
        let Some(target) = target else {
            return Ok(Action::None);
        };
        self.connect(self.revision, target, now)
    }

    /// Earliest eligible retry in the controller's monotonic millisecond clock.
    /// None means wait for a state/inventory event, not a periodic retry.
    pub fn automatic_deadline(&self) -> Option<i64> {
        if !self.inventory_available || !self.automatic_admitted() {
            return None;
        }
        self.devices
            .values()
            .filter_map(|device| {
                if !self.automatic_candidate(device) {
                    return None;
                }
                match self.automatic.get(&device.key) {
                    Some(retry) => (!retry.suppressed).then_some(retry.deadline),
                    None => Some(0),
                }
            })
            .min()
    }

    fn automatic_admitted(&self) -> bool {
        !self.saved.is_empty() && self.links.len() < self.limits.links && self.admitting().is_ok()
    }

    /// A native discovery that never produced Hello must not repeatedly probe
    /// the device. Startup failures before native admission retain bounded retries.
    pub(super) fn retire_automatic(&mut self, link: &Link<K>) {
        if link.ticket.is_none() && link.completion.is_some() {
            if let Some(retry) = self.automatic.get_mut(&link.key) {
                retry.suppress();
            }
        }
    }

    fn automatic_candidate(&self, device: &Device<K, T>) -> bool {
        device.accessory
            && device.availability == Availability::Available
            && !self.links.values().any(|link| link.key == device.key)
    }

    fn automatic_ready(&self, device: &Device<K, T>, now: i64) -> bool {
        self.automatic_candidate(device)
            && self
                .automatic
                .get(&device.key)
                .is_none_or(|retry| retry.ready(now))
    }
}
