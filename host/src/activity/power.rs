//! Auxiliary observation needs logind, without compositor or private DPMS helpers.
use super::*;
use blent_config::host_activity::{SourceCreation, SourceFactory, StartedSource};

pub(super) async fn system_bus(settings: Settings) -> Option<zbus::Connection> {
    tokio::time::timeout(settings.deadline, zbus::Connection::system())
        .await
        .ok()
        .and_then(Result::ok)
}

impl LinuxSource {
    pub(super) async fn connect_power(
        settings: Settings,
        lifetime: SourceLifetime,
    ) -> Result<Self> {
        settings.validate()?;
        let epoch = fresh_epoch()?;
        let actors = Self::discover(
            system_bus(settings).await,
            None,
            settings,
            std::env::var("XDG_SESSION_ID").ok(),
            "",
        )
        .await;
        Self::start(epoch, settings, actors, lifetime)
    }
}

/// Implements the independent source contract with logind power/lock observations.
/// Display capability remains explicitly unsupported; auxiliary policy ignores lock.
pub struct PowerFactory;
impl SourceFactory for PowerFactory {
    fn create(&self, settings: Settings, lifetime: SourceLifetime) -> SourceCreation {
        Box::pin(async move {
            let source = LinuxSource::connect_power(settings, lifetime).await?;
            Ok(StartedSource {
                epoch: source.epoch(),
                source: Box::new(source),
            })
        })
    }
}
