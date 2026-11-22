//! Deadlines are scalar hints only; collection still requires original native
//! completion. Socket/channel readiness remains responsible for live traffic.
use super::*;
impl Server {
    pub(super) fn maintenance_deadline(&self) -> Option<Instant> {
        let now = Instant::now();
        [
            (now < self.accept_after).then_some(self.accept_after),
            self.inventory_deadline(),
            self.controller.expiry_deadline().map(|ms| self.at(ms)),
            self.controller.automatic_deadline().map(|ms| self.at(ms)),
            self.observation_needed().then_some(self.next_observation),
        ]
        .into_iter()
        .flatten()
        .min()
    }
    fn at(&self, milliseconds: i64) -> Instant {
        // Controller deadlines are validated nonnegative i64 milliseconds.
        // Clamp solely to the platform's representable Instant horizon.
        self.origin
            .checked_add(Duration::from_millis(milliseconds as u64))
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400))
    }
    fn inventory_deadline(&self) -> Option<Instant> {
        if self.inventory.is_none() && self.controller.inventory_needed() {
            Some(self.next_inventory)
        } else {
            None
        }
    }
    fn observation_needed(&self) -> bool {
        self.inventory.as_ref().is_some_and(Inventory::finished)
            || self.clients.values().any(|client| client.peer.is_none())
            || self.links.values().any(|link| link.handle.is_none())
            || self.controller.retirement_pending()
    }
}
pub(super) async fn wait(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
