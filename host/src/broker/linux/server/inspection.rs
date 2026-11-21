//! One initial Inspect per admitted client waits for the first settled scan.
//! Client closure discards only its bounded request, never shared link ownership.
use super::*;
impl Server {
    pub(super) fn inspect(&mut self, client: u64, request: u64) -> Result<()> {
        if self.inventory_observed {
            self.snapshot(client, request);
            return Ok(());
        }
        let owner = self
            .clients
            .get_mut(&client)
            .context("unknown inspection client")?;
        if owner.inspection.is_some() {
            self.reject(client, request, Rejection::Busy);
        } else {
            owner.inspection = Some(request);
        }
        Ok(())
    }

    pub(super) fn complete_inspections(&mut self) {
        let pending: Vec<_> = self
            .clients
            .iter_mut()
            .filter_map(|(&client, owner)| owner.inspection.take().map(|request| (client, request)))
            .collect();
        for (client, request) in pending {
            self.snapshot(client, request);
        }
    }
}
