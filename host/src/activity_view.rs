//! Read-only runtime adapter for the independent activity work contract.
use blent_config::host_activity::{ActivityTarget, Publication, WorkObservation};
use std::{future::Future, pin::Pin};
use tokio::sync::watch;

pub type Change = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct View {
    target: ActivityTarget,
    source: Option<watch::Receiver<Publication>>,
}
impl View {
    pub fn new(target: ActivityTarget, source: Option<watch::Receiver<Publication>>) -> Self {
        Self { target, source }
    }
    pub fn begin(&self) -> Option<WorkObservation> {
        WorkObservation::begin(self.target, self.publication(), self.open())
    }
    pub fn permits(&self, original: WorkObservation) -> bool {
        original.permits(self.target, self.publication(), self.open())
    }
    fn publication(&self) -> Option<Publication> {
        self.source.as_ref().map(|source| *source.borrow())
    }
    fn open(&self) -> bool {
        self.source
            .as_ref()
            .is_some_and(|source| source.has_changed().is_ok())
    }
    /// Arm synchronously before reading begin()/permits(); closure or publication
    /// before first poll must still wake this original read-only observation.
    pub fn changed(&self) -> Change {
        let Some(source) = &self.source else {
            return Box::pin(std::future::pending());
        };
        let mut source = source.clone();
        let open = source.has_changed().is_ok();
        source.borrow_and_update();
        Box::pin(async move {
            if open {
                let _ = source.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        })
    }
}

#[cfg(test)]
mod tests;
