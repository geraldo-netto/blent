use super::*;
impl Server {
    /// Stop accepting work, revoke local routes, then observe actual task/helper
    /// retirement. A timeout leaves this Server owning its quarantined records.
    pub async fn shutdown(&mut self, budget: Duration) -> Result<()> {
        ensure!(
            !budget.is_zero() && budget <= Duration::from_secs(60),
            "invalid broker shutdown deadline"
        );
        self.request_stop()?;
        tokio::time::timeout(budget, async {
            self.drain().await?;
            self.authority.close();
            while !self.authority.completion().is_complete() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        Ok(())
    }
    fn request_stop(&mut self) -> Result<()> {
        let clients: Vec<_> = self.clients.keys().copied().collect();
        for client in clients {
            self.close_client(client)?;
        }
        let links: Vec<_> = self.links.keys().copied().collect();
        for link in links {
            self.controller.failed(link)?;
        }
        self.close_requested()
    }
    fn quiet(&self) -> bool {
        self.clients.is_empty()
            && self.links.is_empty()
            && self.routes.is_empty()
            && self.controller.closing().is_empty()
            && !self.authority_busy
            && self.inventory.as_ref().is_none_or(Inventory::retired)
    }
    async fn drain(&mut self) -> Result<()> {
        let mut clock = tokio::time::interval(Duration::from_millis(10));
        loop {
            self.collect_clients();
            self.collect_links()?;
            self.authorize_next()?;
            if self.quiet() {
                return Ok(());
            }
            tokio::select! {
                event = self.authority.next(), if self.authority_busy => self.authority_event(event?)?,
                result = inventory_next(&mut self.inventory) => { self.inventory_result(result)?; }
                _ = clock.tick() => (),
            }
            self.close_requested()?;
        }
    }
}
