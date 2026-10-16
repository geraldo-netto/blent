//! Original desired-work and attachment observations for one pipeline attempt.
use blent::{
    capture_contract::{ProfileLease, ProfileSource, Signal},
    display_work::{Observation, View},
};
use std::{future::Future, sync::Arc};
use tokio::sync::watch;

pub(super) struct Authority {
    work: Arc<View>,
    observation: Observation,
    profile: Option<Box<dyn ProfileLease>>,
    display: watch::Receiver<bool>,
    stop: watch::Receiver<bool>,
}
pub(super) struct Sources {
    work: Arc<View>,
    profile: Option<Arc<dyn ProfileSource>>,
}
impl Sources {
    pub(super) fn new(work: Option<Arc<View>>, profile: Option<Arc<dyn ProfileSource>>) -> Self {
        Self {
            work: work.unwrap_or_else(|| Arc::new(View::new(None, None, false))),
            profile,
        }
    }
    pub(super) fn begin(
        &self,
        display: &watch::Receiver<bool>,
        stop: &watch::Receiver<bool>,
    ) -> Option<Authority> {
        let authority = Authority {
            work: self.work.clone(),
            observation: self.work.begin()?,
            profile: self.profile.as_ref().map(|profile| profile.profile()),
            display: display.clone(),
            stop: stop.clone(),
        };
        authority.current().then_some(authority)
    }
    pub(super) fn changed(&self) -> Signal<'static> {
        let work = self.work.changed();
        let profile = profile_changed(self.profile.as_ref());
        Box::pin(async move {
            tokio::select! { _ = work => {}, _ = profile => {} }
        })
    }
}
impl Authority {
    fn current(&self) -> bool {
        self.work.permits(&self.observation)
            && self.profile.as_ref().is_none_or(|profile| profile.active())
            && *self.display.borrow()
            && self.display.has_changed().is_ok()
            && !*self.stop.borrow()
            && self.stop.has_changed().is_ok()
    }
    fn changed(&self) -> Signal<'_> {
        let activity = self.work.changed();
        let mut display = self.display.clone();
        let mut stop = self.stop.clone();
        display.borrow_and_update();
        stop.borrow_and_update();
        Box::pin(async move {
            tokio::select! {
                _ = activity => {},
                _ = profile_retired(self.profile.as_deref()) => {},
                _ = display.changed() => {},
                _ = stop.changed() => {},
            }
        })
    }
    pub(super) async fn guarded<T>(&self, work: impl Future<Output = T>) -> Option<T> {
        let checked = blent::current_work::guarded(|| self.current(), work);
        tokio::pin!(checked);
        loop {
            let change = self.changed();
            tokio::select! { biased;
                result = &mut checked => return result,
                _ = change => {},
            }
        }
    }
}
async fn profile_retired(profile: Option<&dyn ProfileLease>) {
    match profile {
        Some(profile) => profile.retired().await,
        None => std::future::pending().await,
    }
}
pub(super) fn profile_changed(profile: Option<&Arc<dyn ProfileSource>>) -> Signal<'static> {
    match profile {
        Some(profile) => profile.changed(),
        None => Box::pin(std::future::pending()),
    }
}
