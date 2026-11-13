//! Bounded scheduling hints for remembered-peer discovery. Physical locations
//! select attempts only; the existing pairing/TLS flow remains authoritative.
use super::*;
use blent_config::connection::control::Availability;

impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    /// Unknown inventory cannot establish detach or clear an explicit cutoff.
    pub fn inventory_unavailable(&mut self) -> Result<()> {
        self.editable()?;
        self.devices.clear();
        self.changed()
    }

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

    fn automatic_ready(&self, device: &Device<K, T>, now: i64) -> bool {
        device.availability == Availability::Available
            && !self.links.values().any(|link| link.key == device.key)
            && self
                .automatic
                .get(&device.key)
                .is_none_or(|retry| retry.is_some_and(|deadline| now >= deadline))
    }
}
