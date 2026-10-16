//! Original activity/attachment inputs for one trial; never native retirement.
use blent::capture_contract::ProfileLease;
#[cfg(test)]
use blent::capture_contract::ProfileSource;
#[cfg(test)]
use blent_config::host_activity::ActivityTarget;
use blent_config::host_activity::WorkObservation;

pub(super) struct TrialAuthority<'a> {
    pub(super) activity: &'a blent::activity_view::View,
    pub(super) work: WorkObservation,
    pub(super) profile: Box<dyn ProfileLease>,
}
impl TrialAuthority<'_> {
    // Existing unmanaged compositions retain their explicit unsupported view.
    #[cfg(test)]
    pub(super) fn legacy(profile: &dyn ProfileSource) -> TrialAuthority<'static> {
        static VIEW: std::sync::LazyLock<blent::activity_view::View> =
            std::sync::LazyLock::new(|| {
                blent::activity_view::View::new(ActivityTarget::Display, None)
            });
        TrialAuthority {
            activity: &VIEW,
            work: VIEW.begin().unwrap(),
            profile: profile.profile(),
        }
    }
    pub(super) fn current(&self) -> bool {
        self.activity.permits(self.work) && self.profile.active()
    }
}

pub(super) async fn retired(
    original: &blent::native_scope::Retirement,
    stop: &mut tokio::sync::watch::Receiver<bool>,
) -> bool {
    loop {
        if super::stopped(stop) {
            return false;
        }
        if original.is_complete() {
            return true;
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
            _ = stop.changed() => {},
        }
    }
}

/// Read views of the original trial inputs; no copied mutable settings/flags.
pub(super) struct Inputs<'a> {
    key: &'a super::Key,
    settings: tokio::sync::watch::Receiver<crate::media::EncoderSettings>,
    display: tokio::sync::watch::Receiver<bool>,
    stop: tokio::sync::watch::Receiver<bool>,
}
impl<'a> Inputs<'a> {
    pub(super) fn new(
        key: &'a super::Key,
        settings: &tokio::sync::watch::Receiver<crate::media::EncoderSettings>,
        display: &tokio::sync::watch::Receiver<bool>,
        stop: &tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            key,
            settings: settings.clone(),
            display: display.clone(),
            stop: stop.clone(),
        }
    }

    fn current(&self, authority: &TrialAuthority<'_>) -> bool {
        !super::stopped(&self.stop)
            && self.display.has_changed().is_ok()
            && *self.display.borrow()
            && self.settings.has_changed().is_ok()
            && self.settings.borrow().selection_matches(self.key)
            && authority.current()
    }
    fn poll<T>(
        &self,
        authority: &TrialAuthority<'_>,
        work: std::pin::Pin<&mut impl std::future::Future<Output = T>>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        if !self.current(authority) {
            return std::task::Poll::Ready(None);
        }
        let result = work.poll(cx);
        // Work may make progress at the same instant an original input retires.
        if !self.current(authority) {
            return std::task::Poll::Ready(None);
        }
        result.map(Some)
    }
}

pub(super) async fn guarded<T>(
    inputs: Inputs<'_>,
    authority: &TrialAuthority<'_>,
    work: impl std::future::Future<Output = T>,
) -> Option<T> {
    tokio::pin!(work);
    std::future::poll_fn(|cx| inputs.poll(authority, work.as_mut(), cx)).await
}

#[cfg(test)]
mod tests;
