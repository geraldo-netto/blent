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
        if self.saved.is_empty() || self.links.len() >= self.limits.links {
            return Ok(Action::None);
        }
        if self.admitting().is_err() {
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

    /// A native discovery that never produced Hello must not repeatedly probe
    /// the device. Startup failures before native admission retain bounded retries.
    pub(super) fn retire_automatic(&mut self, link: &Link<K>) {
        if link.ticket.is_none() && link.completion.is_some() {
            if let Some(retry) = self.automatic.get_mut(&link.key) {
                retry.suppress();
            }
        }
    }

    fn automatic_ready(&self, device: &Device<K, T>, now: i64) -> bool {
        device.accessory
            && device.availability == Availability::Available
            && !self.links.values().any(|link| link.key == device.key)
            && self
                .automatic
                .get(&device.key)
                .is_none_or(|retry| retry.ready(now))
    }
}
