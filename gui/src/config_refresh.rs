//! Bounded observations are produced by the existing status worker.
use blent_config::model::FileConfig;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub started: Instant,
    pub config: Arc<FileConfig>,
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
    }
}

impl Snapshot {
    pub fn read(
        source: &impl blent_config::persistence_contract::ObservedSnapshots,
    ) -> Option<Self> {
        let started = Instant::now();
        source.observe().ok().map(|config| Self {
            started,
            config: Arc::new(config),
        })
    }
}

impl crate::App {
    pub(super) fn refresh_config(&mut self, snapshot: Option<&Snapshot>) {
        let Some(snapshot) = snapshot else {
            return;
        };
        if self.save.is_some()
            || self
                .config_observed_after
                .is_some_and(|cutoff| snapshot.started <= cutoff)
        {
            return;
        }
        if let Ok(edited) = self
            .cfg
            .merge_edits(&self.saved_cfg, (*snapshot.config).clone())
        {
            self.cfg = edited;
            self.saved_cfg = (*snapshot.config).clone();
            self.config_observed_after = Some(snapshot.started);
        }
    }
}
