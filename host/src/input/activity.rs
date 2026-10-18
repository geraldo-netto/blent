//! One original authenticated controller publishes read-only display demand.
use anyhow::Result;
use blent_config::{
    display_activity::{Projection, Signal},
    host_activity::Publication,
};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

pub(super) struct Observer {
    source: Option<watch::Receiver<Publication>>,
    projection: Projection,
}
impl Observer {
    pub(super) fn new(source: Option<watch::Receiver<Publication>>) -> Self {
        Self {
            source,
            projection: Projection::default(),
        }
    }
    pub(super) fn update(&mut self) -> Result<Option<Signal>> {
        let Some(source) = &self.source else {
            return Ok(None);
        };
        self.projection
            .observe(Some(*source.borrow()), source.has_changed().is_ok())
    }
    pub(super) fn changed(&self) -> crate::activity_view::Change {
        crate::activity_view::notice(self.source.as_ref())
    }
}
pub(super) fn message(signal: Signal) -> Result<Message> {
    Ok(Message::Text(serde_json::to_string(
        &super::wire::ActivityResponse {
            status: "display_activity",
            display_activity: signal,
        },
    )?))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod socket_tests;
