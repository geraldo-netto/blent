//! Encoder process/task ownership. The supervisor decides when to start and stop.
use super::{process, CaptureConfig};
use crate::media::CodecConfig;
use anyhow::Result;
use blent_config::commands::OwnedChild as Child;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct EncoderProcess {
    pub(super) retirement: Option<Arc<dyn blent::capture_contract::NativeRetirement>>,
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
        let async_depth = super::cli_encoder::supports_async_depth(
            std::ffi::OsStr::new("ffmpeg"),
            encoder,
            &blent::native_scope::Scope::new(self.retirement.clone()),
        )
        .await;
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
        process::retain(&self.retirement, child.completion());
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
            &blent::native_scope::Scope::new(self.retirement.clone()),
        );
        let idle_retirement = idle.as_ref().map(super::idle::Handle::completion);
        let handle = blent::owned_task::Task::spawn(super::cli_encoder::read_loop_with_idle(
            stdout,
            output.tx,
            output.codec_config,
            output.latency,
            codec,
            evidence,
            idle,
        ))?;
        process::retain(&self.retirement, handle.completion());
        Ok(EncoderTask {
            handle,
            idle: idle_retirement,
        })
    }

    #[cfg(feature = "inproc-encoder")]
    pub(super) fn spawn_session(
        &mut self,
        config: &CaptureConfig,
        mode: Option<(u32, u32)>,
        output: EncoderOutput,
    ) -> Result<EncoderTask> {
        process::ensure_retired(&self.pending)?;
        let (w, h) = mode.ok_or_else(|| anyhow::anyhow!("Encoder size unavailable"))?;
        if config.ten_bit {
            tracing::warn!(
                "Native encoder worker uses 8-bit; build without inproc-encoder for 10-bit"
            );
        }
        let job = blent::encoder_worker_contract::Job {
            encoder: config.encoder.clone(),
            width: w,
            height: h,
            fps: config.fps,
            bitrate: config.bitrate,
            quality: config.quality,
            slots: config.raw_slots,
            workers: config.worker_count(),
            device: config.vaapi_device.clone(),
            nonce: crate::encoder_worker::nonce()?,
            parent: std::process::id(),
        };
        let fifo = super::fifo_path_for(config.instance)?;
        let (child, handle) = crate::encoder_worker::spawn(
            job,
            &fifo,
            output.raw_socket,
            crate::encoder_worker::Output {
                retirement: self.retirement.clone(),
                video: output.tx,
                headers: output.codec_config,
                latency: output.latency,
                refresh: output.idr_wanted,
            },
            output.fifo,
        )?;
        self.child = Some(child);
        Ok(EncoderTask { handle, idle: None })
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
    idle: Option<blent_config::retirement::Completion>,
    pub(super) handle: blent::owned_task::Task,
}

impl EncoderTask {
    pub(super) fn abort(&mut self) {
        self.handle.abort();
    }

    pub(super) async fn finish(&mut self, already_finished: bool) -> Result<()> {
        self.handle.abort();
        let budget = std::time::Duration::from_millis(blent_config::retirement_limits::configured(
            "BLENT_ENCODER_STOP_MS",
            1000,
            60_000,
        )? as u64);
        tokio::time::timeout(budget, self.retire(already_finished))
            .await
            .map_err(|_| {
                anyhow::anyhow!("encoder task retirement incomplete; replacement forbidden")
            })?
    }
    async fn retire(&mut self, already_finished: bool) -> Result<()> {
        // The reader's generation guard and its original idle controller must
        // both retire before the supervisor can rotate/reuse the FIFO.
        if !already_finished {
            match (&mut self.handle).await {
                Ok(result) => result?,
                Err(error) if error.is_cancelled() => {}
                Err(error) => return Err(error.into()),
            }
        }
        if let Some(idle) = &self.idle {
            while !idle.is_complete() {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::*;
    struct HeldIdle {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl Drop for HeldIdle {
        fn drop(&mut self) {
            let _ = self.entered.send(());
            let _ = self.release.lock().unwrap().recv();
        }
    }
    struct ReleaseIdle(std::sync::mpsc::Sender<()>);
    impl Drop for ReleaseIdle {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    #[tokio::test]
    async fn t1029_joined_reader_waits_for_original_idle_cleanup() {
        if crate::capture::ownership_tests::isolated("capture::encoding::retirement_tests::t1029_joined_reader_waits_for_original_idle_cleanup") { return; }
        std::env::set_var("BLENT_ENCODER_STOP_MS", "1000");
        let mut idle = blent::owned_task::Task::spawn(std::future::pending()).unwrap();
        let original = idle.completion();
        let (entered_tx, entered) = std::sync::mpsc::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = ReleaseIdle(release);
        idle.keep_alive(HeldIdle {
            entered: entered_tx,
            release: std::sync::Mutex::new(held),
        });
        idle.abort();
        assert!((&mut idle).await.unwrap_err().is_cancelled());
        drop(idle);
        entered
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let mut handle = blent::owned_task::Task::spawn(async { Ok(()) }).unwrap();
        (&mut handle).await.unwrap().unwrap();
        let mut reader = EncoderTask {
            handle,
            idle: Some(original.clone()),
        };
        let pending =
            tokio::time::timeout(std::time::Duration::from_millis(25), reader.finish(true))
                .await
                .is_err();
        assert!(!original.is_complete());
        std::env::set_var("BLENT_ENCODER_STOP_MS", "10");
        let deadline = reader
            .finish(true)
            .await
            .expect_err("T1029 deadline must reject original idle cleanup");
        assert!(deadline.to_string().contains("retirement incomplete"));
        assert!(
            !original.is_complete(),
            "T1029 deadline released original idle work"
        );
        drop(release);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !original.is_complete() {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        reader.finish(true).await.unwrap();
        assert!(pending, "T1029 joined reader ignored original idle cleanup");
    }

    #[tokio::test]
    async fn t814_encoder_cleanup_errors_and_panics_are_not_success() {
        for panic in [false, true] {
            let handle = blent::owned_task::Task::spawn(async move {
                assert!(!panic, "T814 cleanup panic");
                anyhow::bail!("T814 cleanup error")
            })
            .unwrap();
            let mut task = EncoderTask { handle, idle: None };
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            assert!(
                task.finish(false).await.is_err(),
                "T814: encoder cleanup failure erased"
            );
        }
    }
    #[tokio::test]
    async fn t819_async_encoder_task_cancellation_and_completion_are_observed() {
        let mut done = EncoderTask {
            idle: None,
            handle: blent::owned_task::Task::spawn(async { Ok(()) }).unwrap(),
        };
        tokio::task::yield_now().await;
        done.finish(false).await.unwrap();
        let mut pending = EncoderTask {
            idle: None,
            handle: blent::owned_task::Task::spawn(std::future::pending()).unwrap(),
        };
        pending.abort();
        pending.finish(false).await.unwrap();
        pending.finish(true).await.unwrap();
    }
    #[cfg(feature = "inproc-encoder")]
    #[tokio::test]
    async fn t819_native_encoder_process_tracks_start_and_reaped_shutdown() {
        let mut process = EncoderProcess::default();
        assert!(!process.is_running());
        let config = CaptureConfig {
            encoder: "libx264".into(),
            ..Default::default()
        };
        assert_eq!(process.start(&config, (64, 64)).await.unwrap(), (64, 64));
        for explicit in [false, true] {
            process.child =
                Some(Child::spawn(tokio::process::Command::new("/bin/sleep").arg("30")).unwrap());
            assert!(process.is_running());
            if explicit {
                process.shutdown().await;
            } else {
                process.stop();
            }
            assert!(!process.is_running());
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                while !process.pending.as_ref().unwrap().is_complete() {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
        }
        process.shutdown().await;
    }
}
