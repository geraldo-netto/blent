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
}
#[derive(Clone)]
pub struct Observation {
    activity: WorkObservation,
    presence: Option<ViewerPresence>,
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
        }
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
        })
    }
    pub fn permits(&self, original: &Observation) -> bool {
        self.activity.permits(original.activity) && self.continuous(original)
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
        let viewers = self.viewers.as_ref().map(|viewers| viewers.changed());
        Box::pin(async move {
            match viewers {
                Some(viewers) => {
                    tokio::select! { _ = activity => {}, _ = viewers => {} }
                }
                None => activity.await,
            }
        })
    }
}
#[cfg(test)]
mod tests;
