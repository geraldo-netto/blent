//! Cancel producers and retire device routes before a port slot can be reused.
use super::*;

impl Monitor {
    pub(super) fn retire(
        &mut self,
        serial: String,
        instance: u32,
        session: Option<ExtraSession>,
        task: Option<tokio::task::JoinHandle<Mutation>>,
    ) {
        if let Some(session) = &session {
            let _ = session.tablet_tx.send(false);
        }
        let ticket = self
            .slots
            .ticket(&DisplayOwner::Adb(serial.clone()))
            .expect("assigned display ticket");
        assert_eq!(ticket.instance(), instance);
        self.slots.close(ticket).expect("current display ticket");
        self.retiring_slots.insert(instance);
        self.retiring_routes.insert(serial.clone(), instance);
        let adb = self.config.adb.clone();
        let routes = self.routes.remove(&serial);
        self.retiring.spawn(async move {
            if let Some(task) = task {
                let _ = task.await;
            }
            let completion = match session {
                Some(session) => Some(session.stop().await),
                None => None,
            };
            if let Some(routes) = routes {
                if let Err(error) = routes.retire(&adb).await {
                    // Offline cleanup remains bounded; credentials were revoked
                    // before retirement, and foreign routes are never removed.
                    warn!("Could not retire owned routes on {serial}: {error}");
                }
            }
            if let Some(completion) = completion {
                // Bounded logical stop can leave native workers quarantined.
                // Keep this instance reserved while other retirement jobs run.
                while !completion.is_complete() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            (serial, instance, ticket)
        });
    }

    pub(super) fn retired(&mut self, (serial, instance, ticket): RetiredDisplay) {
        if ticket.instance() != instance
            || self.slots.owner(ticket) != Some(&DisplayOwner::Adb(serial.clone()))
        {
            return;
        }
        if self.slots.retired(ticket).is_err() {
            return;
        }
        self.retiring_routes.remove(&serial);
        self.retiring_slots.remove(&instance);
    }

    pub(super) fn remove_assignment(&mut self, serial: &str) {
        let ready = self.ready.remove(serial);
        self.attachments.remove(serial);
        let task = self.mutations.retire(serial);
        self.token_retries.remove(serial);
        let pending = self.pending.remove(serial);
        let assigned = ready || pending.is_some() || task.is_some();
        let session = pending
            .and_then(|pending| pending.session)
            .or_else(|| self.extras.remove(serial));
        let instance = self
            .slots
            .ticket(&DisplayOwner::Adb(serial.into()))
            .map_or(0, DisplayTicket::instance);
        if (assigned || session.is_some()) && !self.retiring_routes.contains_key(serial) {
            self.retire(serial.into(), instance, session, task);
        }
    }
}

#[cfg(test)]
mod tests;
