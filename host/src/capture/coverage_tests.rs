//! T497: supervisor event contracts with owned idle processes, never real capture.
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tokio::process::Command;

const TEST: &str =
    "capture::coverage_tests::t497_supervisor_distinguishes_notifications_restarts_and_failures";

struct Fixture {
    manager: CaptureManager,
    run: CaptureRun,
    settings: watch::Sender<EncoderSettings>,
    display: watch::Sender<bool>,
    _shutdown: watch::Sender<bool>,
}

#[derive(Default)]
struct ReaderProofs(std::sync::Mutex<Vec<blent_config::retirement::Completion>>);

impl blent::capture_contract::NativeRetirement for ReaderProofs {
    fn retain(&self, original: blent_config::retirement::Completion) {
        let mut proofs = self.0.lock().unwrap();
        assert!(proofs.len() < 2, "T1708 replaced the original reader");
        proofs.push(original);
    }
}

impl ReaderProofs {
    fn assert_reader_retired(&self, retired: bool) {
        let proofs = self.0.lock().unwrap();
        assert_eq!(
            proofs.len(),
            2,
            "T1708 requires original encoder and reader"
        );
        assert!(!proofs[0].is_complete(), "T1708 encoder exited early");
        assert_eq!(proofs[1].is_complete(), retired, "T1708 reader settlement");
    }
}

fn idle_child() -> blent_config::commands::OwnedChild {
    blent_config::commands::OwnedChild::spawn(
        Command::new("/bin/sleep")
            .arg("60")
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true),
    )
    .unwrap()
}

impl Fixture {
    fn new() -> Self {
        let manager = CaptureManager::new(CaptureConfig {
            encoder: "libx264".into(),
            width: 64,
            height: 64,
            instance: u32::MAX,
            adaptive_idle: false,
            helper_path: "/never/execute/an/evdi/helper".into(),
            ..Default::default()
        });
        let settings = EncoderSettings {
            encoder: "libx264".into(),
            fps: 60,
            bitrate: 20_000,
            width: 64,
            height: 64,
            quality: crate::config::DEFAULT_QUALITY,
            width_mm: manager.config.width_mm,
            height_mm: manager.config.height_mm,
            stream_scale: 1,
            geometry_ready: true,
            decoders: None,
            decoder_epoch: 0,
            selection: None,
        };
        let (settings, settings_rx) = watch::channel(settings);
        let (display, display_rx) = watch::channel(true);
        let (shutdown, shutdown_rx) = watch::channel(false);
        let run = CaptureRun {
            settings_rx,
            display_rx,
            shutdown_rx,
            mode_rx: manager.helper.mode_rx.clone(),
            stream_rx: manager.helper.stream_rx.clone(),
            fifo_reset_rx: manager.helper.fifo_reset_rx.clone(),
            backoff_ms: 0,
            explained_evdi: false,
            pipeline_started_at: Instant::now(),
            encoder_mode: Some((64, 64)),
        };
        Self {
            manager,
            run,
            settings,
            display,
            _shutdown: shutdown,
        }
    }

    fn children(&mut self) {
        self.manager.helper.child = Some(idle_child());
        self.manager.helper.card = Some(u32::MAX);
        self.manager.encoder.child = Some(idle_child());
    }

    async fn change_mode(&mut self, stream: bool) {
        self.children();
        let helper = self.manager.helper.child.as_ref().unwrap().completion();
        let proofs = Arc::new(ReaderProofs::default());
        self.manager.encoder.retirement = Some(proofs.clone());
        let (video, _receiver) = crate::video_queue::channel(8, Default::default());
        let stream_tx = self.manager.helper.stream_tx.clone();
        let mode_tx = self.manager.helper.mode_tx.clone();
        let video = video.into();
        let mut task = Box::pin(self.manager.run_encoder_session(&video, &mut self.run));
        assert!(futures_util::poll!(&mut task).is_pending());
        for size in [64, 128] {
            if stream {
                stream_tx.send(Some((size, size))).unwrap();
            } else {
                mode_tx
                    .send(Some(DetectedMode {
                        width: size,
                        height: size,
                        refresh: 60,
                    }))
                    .unwrap();
            }
            if size == 64 {
                assert!(
                    futures_util::poll!(&mut task).is_pending(),
                    "unchanged dimensions restarted encoder"
                );
                proofs.assert_reader_retired(false);
                assert!(!helper.is_complete());
            }
        }
        let changes = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert!(changes.mode_changed);
        assert!(!changes.settings_changed);
        assert!(changes.keep_helper());
        proofs.assert_reader_retired(true);
        assert!(!helper.is_complete());
        self.manager.shutdown().await;
        assert!(helper.is_complete());
        assert!(proofs
            .0
            .lock()
            .unwrap()
            .iter()
            .all(|proof| proof.is_complete()));
    }

    async fn display_notifications(&mut self) {
        self.children();
        let (video, _receiver) = crate::video_queue::channel(8, Default::default());
        let video = video.into();
        let sources = super::activity::Sources::new(None, None);
        let authority = sources
            .begin(&self.run.display_rx, &self.run.shutdown_rx)
            .unwrap();
        let mut task =
            Box::pin(authority.guarded(self.manager.run_encoder_session(&video, &mut self.run)));
        assert!(futures_util::poll!(&mut task).is_pending());
        self.display.send(true).unwrap();
        assert!(
            futures_util::poll!(&mut task).is_pending(),
            "same active display restarted encoder"
        );
        self.settings
            .send_modify(|settings| settings.decoder_epoch += 1);
        assert!(
            futures_util::poll!(&mut task).is_pending(),
            "metadata-only change restarted encoder"
        );
        self.display.send(false).unwrap();
        let changes = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap();
        assert!(
            changes.is_none(),
            "T497/T1683 display loss must revoke the guarded attempt"
        );
        self.manager.shutdown().await;
    }

    async fn failed_start(&mut self) {
        self.manager.config.encoder = "invalid-encoder".into();
        assert!(!self.manager.ensure_session_encoder(&mut self.run).await);
        assert!(!self.manager.encoder.is_running());
        self.display.send(false).unwrap();
        assert!(!self.manager.ensure_session_encoder(&mut self.run).await);
        assert!(crate::test_logging::text().contains("Failed to start encoder"));
    }

    fn explain_once(&mut self) {
        self.run.explain_evdi_failure_with(|| None);
        assert!(!self.run.explained_evdi);
        self.run
            .explain_evdi_failure_with(|| Some("T497 missing EVDI fixture".into()));
        self.run
            .explain_evdi_failure_with(|| panic!("explained failure must not be re-queried"));
        assert!(self.run.explained_evdi);
        self.run.explain_evdi_failure();
        assert_eq!(
            crate::test_logging::text()
                .matches("T497 missing EVDI fixture")
                .count(),
            1
        );
    }
}

#[tokio::test]
async fn t497_supervisor_distinguishes_notifications_restarts_and_failures() {
    if std::env::var_os("BLENT_T497_CAPTURE_STATE").is_none() {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env("BLENT_T497_CAPTURE_STATE", "1")
            .env("HOME", root.path())
            .env("XDG_RUNTIME_DIR", root.path())
            .env("PATH", root.path())
            .env("XDG_CURRENT_DESKTOP", "T497 fixture")
            .env("XDG_SESSION_TYPE", "x11")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    crate::test_logging::enable();
    for stream in [false, true] {
        Fixture::new().change_mode(stream).await;
    }
    Fixture::new().display_notifications().await;
    let mut fixture = Fixture::new();
    fixture.failed_start().await;
    fixture.explain_once();
    assert!(
        !fifo_path_for(u32::MAX).unwrap().exists(),
        "supervisor fixture started capture"
    );
}

#[tokio::test]
async fn t1683_authority_loss_preempts_inner_session_teardown() {
    for shutdown in [false, true] {
        let mut fixture = Fixture::new();
        fixture.children();
        let helper = fixture.manager.helper.child.as_ref().unwrap().completion();
        let encoder = fixture.manager.encoder.child.as_ref().unwrap().completion();
        let sources = super::activity::Sources::new(None, None);
        let authority = sources
            .begin(&fixture.run.display_rx, &fixture.run.shutdown_rx)
            .unwrap();
        let (video, _received) = crate::video_queue::channel(8, Default::default());
        let video = video.into();
        let mut session = Box::pin(
            authority.guarded(
                fixture
                    .manager
                    .run_encoder_session(&video, &mut fixture.run),
            ),
        );
        assert!(futures_util::poll!(&mut session).is_pending());
        if shutdown {
            fixture._shutdown.send(true).unwrap();
        } else {
            fixture.display.send(false).unwrap();
        }
        assert!(
            session.await.is_none(),
            "T1683 obsolete inner teardown won authority loss"
        );
        fixture.manager.shutdown().await;
        assert!(helper.is_complete());
        assert!(encoder.is_complete());
    }
}
