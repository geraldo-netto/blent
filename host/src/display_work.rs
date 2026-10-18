//! Desired display work from original machine and viewer observations.
//! No acquisition, connection grant or native retirement authority.
use crate::{
    activity_view,
    capture_contract::{Signal, ViewerActivity, ViewerPresence},
};
use blent_config::host_activity::{ActivityTarget, Publication, WorkObservation};
use std::sync::Arc;
use tokio::sync::watch;

pub struct View {
    activity: activity_view::View,
    viewers: Option<Arc<dyn ViewerActivity>>,
    pause_without_viewer: bool,
    gate: Option<crate::display_quiescence::Source>,
}
#[derive(Clone)]
pub struct Observation {
    activity: WorkObservation,
    presence: Option<ViewerPresence>,
    gate: Option<crate::display_quiescence::Permit>,
}
impl View {
    pub fn new(
        source: Option<watch::Receiver<Publication>>,
        viewers: Option<Arc<dyn ViewerActivity>>,
        pause_without_viewer: bool,
    ) -> Self {
        Self {
            activity: activity_view::View::new(ActivityTarget::Display, source),
            viewers,
            pause_without_viewer,
            gate: None,
        }
    }
    pub fn with_gate(mut self, gate: crate::display_quiescence::Source) -> Self {
        self.gate = Some(gate);
        self
    }
    pub fn begin(&self) -> Option<Observation> {
        let presence = self.presence();
        if presence
            .as_ref()
            .is_some_and(|presence| !presence.present())
        {
            return None;
        }
        Some(Observation {
            activity: self.activity.begin()?,
            presence,
            gate: self.gate_observation()?,
        })
    }
    pub fn permits(&self, original: &Observation) -> bool {
        self.activity.permits(original.activity)
            && self.continuous(original)
            && self.gate_continues(original)
    }
    fn gate_observation(&self) -> Option<Option<crate::display_quiescence::Permit>> {
        match &self.gate {
            Some(gate) => Some(Some(gate.begin()?)),
            None => Some(None),
        }
    }
    fn gate_continues(&self, original: &Observation) -> bool {
        match (&self.gate, &original.gate) {
            (Some(gate), Some(original)) => gate.permits(original),
            (None, None) => true,
            _ => false,
        }
    }
    fn presence(&self) -> Option<ViewerPresence> {
        if !self.pause_without_viewer {
            return None;
        }
        self.viewers.as_ref().and_then(|viewers| viewers.presence())
    }
    fn continuous(&self, original: &Observation) -> bool {
        match (self.presence(), &original.presence) {
            (Some(current), Some(original)) => current.continues(original),
            (None, None) => true,
            _ => false,
        }
    }
    pub fn changed(&self) -> Signal<'static> {
        let activity = self.activity.changed();
        let gate = self.gate.as_ref().map(|gate| gate.changed());
        let viewers = self.viewers.as_ref().map(|viewers| viewers.changed());
        Box::pin(async move {
            tokio::select! {
                _ = activity => {},
                _ = optional_notice(viewers) => {},
                _ = optional_notice(gate) => {},
            }
        })
    }
}
async fn optional_notice(notice: Option<Signal<'static>>) {
    match notice {
        Some(notice) => notice.await,
        None => std::future::pending().await,
    }
}
#[cfg(test)]
mod tests;
