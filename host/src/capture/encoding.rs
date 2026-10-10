//! Encoder process/task ownership. The supervisor decides when to start and stop.
use super::{process, CaptureConfig};
use crate::media::CodecConfig;
use anyhow::Result;
use blent_config::commands::OwnedChild as Child;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct EncoderProcess {
    pub(super) child: Option<Child>,
    pub(super) pending: Option<blent_config::retirement::Completion>,
    #[cfg(not(feature = "inproc-encoder"))]
    pub(super) gpu: super::gpu::Adapter,
}

pub(super) struct EncoderOutput {
    #[cfg(feature = "inproc-encoder")]
    pub(super) fifo: Option<super::fifo::Owned>,
    #[cfg(feature = "inproc-encoder")]
    pub(super) raw_socket: Option<crate::raw_socket::Socket>,
    pub(super) tx: blent::capture_contract::VideoOutput,
    pub(super) codec_config: CodecConfig,
    pub(super) latency: crate::latency::LatencyTracker,
    #[cfg_attr(not(feature = "inproc-encoder"), allow(dead_code))]
    pub(super) idr_wanted: Arc<std::sync::atomic::AtomicBool>,
}

impl EncoderProcess {
    pub(super) fn is_running(&self) -> bool {
        self.child.is_some()
    }

    #[cfg(not(feature = "inproc-encoder"))]
    pub(super) async fn start(
        &mut self,
        config: &CaptureConfig,
        mode: (u32, u32),
    ) -> Result<(u32, u32)> {
        let encoder = crate::config::ffmpeg_encoder_name(&config.encoder);
        let async_depth =
            super::cli_encoder::supports_async_depth(std::ffi::OsStr::new("ffmpeg"), encoder).await;
        self.start_with(config, mode, async_depth, |mut command| {
            Child::spawn(&mut command)
        })
    }

    #[cfg(feature = "inproc-encoder")]
    pub(super) async fn start(
        &mut self,
        config: &CaptureConfig,
        mode: (u32, u32),
    ) -> Result<(u32, u32)> {
        crate::config::validate_encoder_for_build(&config.encoder)?;
        Ok(mode)
    }

    #[cfg(not(feature = "inproc-encoder"))]
    pub(super) fn start_with(
        &mut self,
        config: &CaptureConfig,
        mode: (u32, u32),
        async_depth_supported: bool,
        spawn: impl FnOnce(tokio::process::Command) -> std::io::Result<Child>,
    ) -> Result<(u32, u32)> {
        use anyhow::Context;
        process::ensure_retired(&self.pending)?;
        let adapter = super::cli_encoder::CliEncoder { config };
        let (w, h) = mode;
        adapter.log_encoder_dimensions(w, h);
        let child = spawn(adapter.encoder_command(w, h, async_depth_supported)?)
            .context("Failed to spawn ffmpeg encoder")?;
        tracing::info!("Encoder started (PID: {})", child.id().unwrap_or(0));
        self.child = Some(child);
        Ok(mode)
    }

    #[cfg(not(feature = "inproc-encoder"))]
    pub(super) fn spawn_session(
        &mut self,
        config: &CaptureConfig,
        mode: Option<(u32, u32)>,
        output: EncoderOutput,
    ) -> Result<EncoderTask> {
        let stdout = self
            .child
            .as_mut()
            .unwrap()
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Encoder has no stdout"))?;
        let codec = crate::media::Codec::from_encoder(&config.encoder);
        // Unknown dimensions cannot certify a matching automatic trial.
        let (w, h) = mode.unwrap_or((0, 0));
        let evidence = output.latency.encoder_started_with_budget(
            &config.encoder,
            (w, h, config.fps, config.bitrate, config.quality),
            config.decoder.clone(),
            if config.encoder == "libx264" {
                config.worker_count()
            } else {
                0
            },
            self.child.as_ref().and_then(|child| child.id()),
        );
        let idle = super::idle::start(
            config,
            evidence.clone(),
            output.latency.clone(),
            output.tx.viewers(),
        );
        let handle = blent::owned_task::Task::spawn(super::cli_encoder::read_loop_with_idle(
            stdout,
            output.tx,
            output.codec_config,
            output.latency,
            codec,
            evidence,
            idle,
        ))?;
        Ok(EncoderTask { handle })
    }

    #[cfg(feature = "inproc-encoder")]
    pub(super) fn spawn_session(
        &mut self,
        config: &CaptureConfig,
        mode: Option<(u32, u32)>,
        output: EncoderOutput,
    ) -> Result<EncoderTask> {
        let (w, h) = mode.expect("encoder size was selected before starting");
        let (name, fps, bitrate, quality) = (
            config.encoder.clone(),
            config.fps,
            config.bitrate,
            config.quality,
        );
        if config.ten_bit {
            tracing::warn!(
                "10-bit is not supported by the in-process encoder — \
                encoding 8-bit. Build without --features inproc-encoder for 10-bit."
            );
        }
        let fifo = super::fifo_path_for(config.instance)?;
        let raw_slots = config.raw_slots;
        let workers = config.worker_count();
        let vaapi_device = config.vaapi_device.clone();
        let stop = crate::encoder_io::StopSignal::new()?;
        let stopc = stop.clone();
        let handle = blent::owned_task::Task::spawn_blocking(move || {
            let _fifo = output.fifo;
            crate::encoder::run(
                &fifo,
                &name,
                w,
                h,
                fps,
                bitrate,
                quality,
                output.tx,
                output.codec_config,
                output.idr_wanted,
                stopc,
                output.latency,
                output.raw_socket,
                raw_slots,
                workers,
                &vaapi_device,
            )
        })?;
        Ok(EncoderTask { handle, stop })
    }

    pub(super) fn stop(&mut self) {
        #[cfg(not(feature = "inproc-encoder"))]
        self.gpu.stopped();
        if let Some(mut child) = self.child.take() {
            self.pending = Some(child.completion());
            let _ = child.start_kill();
        }
    }

    pub(super) async fn shutdown(&mut self) {
        #[cfg(not(feature = "inproc-encoder"))]
        self.gpu.stopped();
        if let Some(child) = self.child.take() {
            self.pending = Some(child.completion());
            process::terminate(child, "ffmpeg").await;
        }
    }
}

pub(super) struct EncoderTask {
    pub(super) handle: blent::owned_task::Task,
    #[cfg(feature = "inproc-encoder")]
    stop: Arc<crate::encoder_io::StopSignal>,
}

impl EncoderTask {
    pub(super) fn abort(&mut self) {
        #[cfg(feature = "inproc-encoder")]
        self.stop.request();
        self.handle.abort();
    }

    pub(super) async fn finish(&mut self, already_finished: bool) -> Result<()> {
        #[cfg(feature = "inproc-encoder")]
        self.stop.request();
        #[cfg(not(feature = "inproc-encoder"))]
        self.handle.abort();
        // Observe task destruction before rebuilding: the packetizer's
        // generation guard must retire every queued old access unit first.
        if !already_finished {
            let budget = std::time::Duration::from_millis(
                blent_config::retirement_limits::configured("BLENT_ENCODER_STOP_MS", 1000, 60_000)?
                    as u64,
            );
            let outcome = tokio::time::timeout(budget, &mut self.handle)
                .await
                .map_err(|_| {
                    anyhow::anyhow!("encoder task retirement incomplete; replacement forbidden")
                })?;
            match outcome {
                Ok(result) => result?,
                Err(error) if error.is_cancelled() => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.handle.abort();
        Ok(())
    }
}

// Blocking tasks cannot be aborted. Retire their reader on cancellation too.
#[cfg(feature = "inproc-encoder")]
impl Drop for EncoderTask {
    fn drop(&mut self) {
        self.stop.request();
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::*;
    #[tokio::test]
    async fn t814_encoder_cleanup_errors_and_panics_are_not_success() {
        for panic in [false, true] {
            let handle = blent::owned_task::Task::spawn(async move {
                assert!(!panic, "T814 cleanup panic");
                anyhow::bail!("T814 cleanup error")
            })
            .unwrap();
            let mut task = EncoderTask {
                handle,
                #[cfg(feature = "inproc-encoder")]
                stop: crate::encoder_io::StopSignal::new().unwrap(),
            };
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            assert!(
                task.finish(false).await.is_err(),
                "T814: encoder cleanup failure erased"
            );
        }
    }
}
