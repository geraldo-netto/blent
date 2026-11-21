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

    pub(super) async fn quiesce_and_publish(&mut self) -> anyhow::Result<()> {
        self.quiesce_recovering().await;
        self.acknowledge_quiescent();
        Ok(())
    }

    /// Only the capture owner may use its still-quiescent native generation as
    /// evidence for a pause arriving during or after original cleanup.
    pub(super) fn acknowledge_quiescent(&self) {
        let Some(pause) = self.pending_pause() else {
            return;
        };
        // Recovery returned only after an exact false ACK or original child AND
        // stdout settlement, with the encoder's native generation also retired.
        self.quiescence
            .as_ref()
            .expect("original capture participant")
            .acknowledge(&pause);
    }

    pub(super) async fn pause_requested(
        &mut self,
        stop: &watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        let Some(_) = self.pending_pause() else {
            return Ok(());
        };
        let mut stopped = stop.clone();
        if *stopped.borrow() || stopped.has_changed().is_err() {
            return Ok(());
        }
        tokio::select! { biased;
            _ = stopped.wait_for(|stopped| *stopped) => Ok(()),
            result = self.quiesce_and_publish() => result,
        }
    }
}

#[cfg(test)]
mod tests;
