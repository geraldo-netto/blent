//! Retained-monitor quiescence. Desired-work policy is owned by the supervisor.
use super::{
    encoding::{EncoderProcess, EncoderTask},
    helper::HelperProcess,
    CaptureManager,
};

impl CaptureManager {
    /// The reader result must not have been separately consumed by the caller.
    /// Retries after cancellation retain original control and native cleanup.
    pub(super) async fn quiesce_display(
        &mut self,
        mut reader: Option<&mut EncoderTask>,
    ) -> anyhow::Result<()> {
        self.encoder.stop();
        if let Some(reader) = reader.as_mut() {
            reader.abort();
        }
        // Stop the FIFO reader/child concurrently with writer quiescence: a
        // full FIFO must not keep the original pause acknowledgement blocked.
        let (paused, joined) = tokio::join!(pause_helper(&mut self.helper), retire_reader(reader));
        retired(&self.encoder).await;
        joined?;
        paused?;
        self.reset_codec();
        if self.helper.is_running() {
            self.helper.rotate_fifo()?;
        }
        Ok(())
    }
    pub(super) fn reset_codec(&self) {
        self.codec_config.publish(None);
        self.idr_wanted
            .store(true, std::sync::atomic::Ordering::Release);
    }
}
async fn pause_helper(helper: &mut HelperProcess) -> anyhow::Result<()> {
    if helper.is_running() {
        helper.set_capture_active(false).await?;
    }
    Ok(())
}
async fn retire_reader(reader: Option<&mut EncoderTask>) -> anyhow::Result<()> {
    if let Some(reader) = reader {
        reader.retire(false).await?;
    }
    Ok(())
}
async fn retired(encoder: &EncoderProcess) {
    while encoder.ensure_retired().is_err() {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
