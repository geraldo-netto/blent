//! One guarded pipeline attempt at a time, with retained monitor pause.
use super::{activity::Sources, CaptureManager, CaptureRun};
use anyhow::Result;
use blent::capture_contract::{Signal, VideoOutput};

impl CaptureManager {
    pub(super) async fn follow_capture(
        &mut self,
        output: &VideoOutput,
        run: &mut CaptureRun,
    ) -> Result<()> {
        let sources = Sources::new(self.display_work.clone(), self.profile.clone());
        // Workflow phase only; acquisitions still require exact native proofs.
        let mut quiescent = false;
        loop {
            if run.stopped() {
                self.shutdown().await;
                return Ok(());
            }
            if self
                .capture_step(output, run, &sources, &mut quiescent)
                .await?
            {
                return Ok(());
            }
        }
    }
    async fn capture_step(
        &mut self,
        output: &VideoOutput,
        run: &mut CaptureRun,
        sources: &Sources,
        quiescent: &mut bool,
    ) -> Result<bool> {
        if run.inactive() {
            self.apply_stream_settings(run).await;
            *quiescent = false;
            return Ok(self.idle_until_screen_change(run).await);
        }
        let notice = sources.changed();
        if let Some(authority) = sources.begin(&run.display_rx, &run.shutdown_rx) {
            let needs_pause = !*quiescent;
            *quiescent = false;
            return match authority
                .guarded(self.capture_attempt(output, run, needs_pause))
                .await
            {
                Some(result) => result,
                None => {
                    *quiescent = self.pause_capture(run).await?;
                    Ok(false)
                }
            };
        }
        if !*quiescent {
            *quiescent = self.pause_capture(run).await?;
        }
        wait_demand(run, notice).await;
        Ok(false)
    }
    async fn capture_attempt(
        &mut self,
        output: &VideoOutput,
        run: &mut CaptureRun,
        needs_pause: bool,
    ) -> Result<bool> {
        self.await_replacement(needs_pause).await?;
        self.apply_stream_settings(run).await;
        if run.inactive() || !self.prepare_capture_pipeline(run).await {
            return Ok(false);
        }
        let Some(changes) = self.run_encoder_session(output, run).await? else {
            return Ok(true);
        };
        run.adjust_backoff(changes);
        self.finish_encoder_session(changes, run).await?;
        Ok(false)
    }
    async fn await_replacement(&mut self, needs_pause: bool) -> Result<()> {
        if needs_pause && self.display_work.is_some() && self.helper.is_running() {
            self.quiesce_display(None).await?;
        } else {
            self.encoder.stop();
            while self.encoder.ensure_retired().is_err() {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
        if !self.helper.is_running() {
            while self.helper.restart_ready().is_err() {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
        self.reset_codec();
        Ok(())
    }
    async fn pause_capture(&mut self, run: &mut CaptureRun) -> Result<bool> {
        let result = Self::while_active(
            &mut run.display_rx,
            &mut run.shutdown_rx,
            self.quiesce_display(None),
        )
        .await;
        match result {
            Some(result) => result.map(|_| true),
            None => Ok(false),
        }
    }
    pub(super) async fn resume_capture(&mut self, run: &mut CaptureRun) -> bool {
        if self.display_work.is_none() {
            return true;
        }
        let result = Self::while_settings_current(
            &mut run.settings_rx,
            &mut run.display_rx,
            &mut run.shutdown_rx,
            self.helper.set_capture_active(true),
            false,
        )
        .await;
        match result {
            Some(Ok(_)) => true,
            Some(Err(error)) => {
                tracing::warn!(%error, "Capture activity resume failed; retiring the original helper");
                self.helper.terminate().await;
                self.encoder.shutdown().await;
                run.back_off().await;
                false
            }
            None => false,
        }
    }
}
impl CaptureRun {
    fn inactive(&self) -> bool {
        !*self.display_rx.borrow() || !self.settings_rx.borrow().geometry_ready
    }
}
async fn wait_demand(run: &mut CaptureRun, notice: Signal<'static>) {
    tokio::select! {
        _ = run.settings_rx.changed() => {},
        _ = run.display_rx.changed() => {},
        _ = run.shutdown_rx.changed() => {},
        _ = notice => {},
    }
}
