use super::{commands::close, *};
impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    /// Attach before the first asynchronous native operation. Cancellation can
    /// already have happened; its tombstone still must retain this completion.
    pub fn attach(&mut self, connection: u64, completion: Completion) -> Result<()> {
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            link.completion.is_none() && !link.finished,
            "native owner already settled"
        );
        link.completion = Some(completion);
        Ok(())
    }
    /// Adapter proves startup failed before creating ANY native resource.
    pub fn start_failed(&mut self, connection: u64) -> Result<()> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            link.completion.is_none() && !link.finished,
            "native owner already started"
        );
        close(link);
        link.finished = true;
        self.changed()
    }
    pub fn failed(&mut self, connection: u64) -> Result<()> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        close(link);
        self.changed()
    }
    pub fn quarantine(&mut self, connection: u64) -> Result<()> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(
            matches!(link.state, State::Closing | State::Quarantined),
            "link still active"
        );
        link.state = State::Quarantined;
        self.changed()
    }
    /// The owning adapter task has returned and all its local bridges retired.
    /// This alone does not release native admission: Completion is also required.
    pub fn native_finished(&mut self, connection: u64) -> Result<()> {
        self.editable()?;
        let link = self
            .links
            .get_mut(&connection)
            .ok_or_else(|| anyhow::anyhow!("unknown link"))?;
        ensure!(link.completion.is_some(), "native owner not attached");
        close(link);
        link.finished = true;
        self.changed()
    }
    pub fn tick(&mut self, now: i64) -> Result<Vec<u64>> {
        self.editable()?;
        ensure!(now >= 0, "invalid monotonic time");
        let mut expired = Vec::new();
        for (&id, link) in &mut self.links {
            if expires(link, now) {
                close(link);
                expired.push(id);
            }
        }
        if !expired.is_empty() {
            self.changed()?;
        }
        Ok(expired)
    }
    pub fn collect_retired(&mut self) -> Result<Vec<u64>> {
        self.editable()?;
        let retired: Vec<_> = self
            .links
            .iter()
            .filter_map(|(&id, link)| ready(link).then_some(id))
            .collect();
        for id in &retired {
            let mut link = self.links.remove(id).unwrap();
            if let Some(ticket) = link.ticket {
                link.pairing.as_mut().unwrap().retired(&ticket)?;
            }
        }
        if !retired.is_empty() {
            self.changed()?;
        }
        Ok(retired)
    }
}
fn expires<K>(link: &Link<K>, now: i64) -> bool {
    matches!(
        link.state,
        State::Discovering | State::Pending | State::Approved
    ) && now >= link.deadline
}
fn ready<K>(link: &Link<K>) -> bool {
    matches!(link.state, State::Closing | State::Quarantined)
        && link.finished
        && link.pairing.is_some()
        && link.completion.as_ref().is_none_or(Completion::is_complete)
}
