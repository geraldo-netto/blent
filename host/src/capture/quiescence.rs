//! Adapter receipts follow original native proofs, never logical task return.
use super::CaptureManager;
use blent::display_quiescence::Pause;
use tokio::sync::watch;

impl CaptureManager {
    pub(super) fn pending_pause(&self) -> Option<Pause> {
        self.quiescence
            .as_ref()
            .and_then(|participant| participant.pending())
    }

    pub(super) async fn quiesce_and_publish(&mut self, pause: Option<Pause>) -> anyhow::Result<()> {
        self.quiesce_recovering().await;
        let Some(pause) = pause else {
            return Ok(());
        };
        // Recovery returned only after an exact false ACK or original child AND
        // stdout settlement, with the encoder's native generation also retired.
        self.quiescence
            .as_ref()
            .expect("original capture participant")
            .acknowledge(&pause);
        Ok(())
    }

    pub(super) async fn pause_requested(
        &mut self,
        stop: &watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        let Some(pause) = self.pending_pause() else {
            return Ok(());
        };
        let mut stopped = stop.clone();
        if *stopped.borrow() || stopped.has_changed().is_err() {
            return Ok(());
        }
        tokio::select! { biased;
            _ = stopped.wait_for(|stopped| *stopped) => Ok(()),
            result = self.quiesce_and_publish(Some(pause)) => result,
        }
    }
}

#[cfg(test)]
mod tests;
