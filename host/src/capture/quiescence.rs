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
        self.quiesce_display(None).await?;
        let Some(pause) = pause else {
            return Ok(());
        };
        // A retained helper supplied its exact false ACK above. If the original
        // helper was terminated/cancelled instead, await child AND stdout proofs.
        while !self.helper.is_running() && self.helper.restart_ready().is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
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
