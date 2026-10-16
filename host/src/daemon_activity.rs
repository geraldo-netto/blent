//! One daemon owner; sessions receive only its original read-only publication.
use blent::activity_follow::Following;
use blent_config::host_activity::{Settings, SourceFactory};
use std::{sync::Arc, time::Duration};

pub(super) fn start(
    factory: Arc<dyn SourceFactory>,
    preferences: Settings,
) -> anyhow::Result<Following> {
    Following::start(factory, Default::default(), preferences)
}
pub(super) async fn retire(source: Following) {
    source.stop();
    while !source.retired() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(test)]
mod tests;
