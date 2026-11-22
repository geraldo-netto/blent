//! T1682: current retry delay, native retirement and cancellation contracts.
use super::*;
use crate::capture::{ownership_tests, SessionChanges, RECONNECT_DELAY_MS};
use std::time::Duration;
use tokio::sync::watch;

struct Fixture {
    manager: CaptureManager,
    run: CaptureRun,
    display: watch::Sender<bool>,
    stop: watch::Sender<bool>,
}
impl Fixture {
    fn new() -> Self {
        let manager = CaptureManager::new(Default::default());
        let mut run = ownership_tests::restart_run(&manager);
        let (display, display_rx) = watch::channel(true);
        let (stop, shutdown_rx) = watch::channel(false);
        run.display_rx = display_rx;
        run.shutdown_rx = shutdown_rx;
        run.backoff_ms = RECONNECT_DELAY_MS;
        Self {
            manager,
            run,
            display,
            stop,
        }
    }
    async fn recover(&mut self, changes: SessionChanges) -> Duration {
        let start = tokio::time::Instant::now();
        self.manager
            .recover_encoder_session(changes, &mut self.run)
            .await
            .unwrap();
        start.elapsed()
    }
}
fn crash() -> SessionChanges {
    SessionChanges {
        settings_changed: false,
        mode_changed: false,
        display_dropped: false,
        fifo_reset: false,
        gpu_fallback: false,
    }
}

#[tokio::test(start_paused = true)]
async fn t1682_crash_waits_current_delay_before_growing_to_cap() {
    let mut f = Fixture::new();
    for (wait, next) in [
        (2000, 4000),
        (4000, 8000),
        (8000, 16000),
        (16000, 30000),
        (30000, 30000),
        (30000, 30000),
    ] {
        f.run.pipeline_started_at = std::time::Instant::now();
        assert_eq!(f.recover(crash()).await, Duration::from_millis(wait));
        assert_eq!(f.run.backoff_ms, next);
    }
}

#[tokio::test(start_paused = true)]
async fn t1682_stable_session_resets_delay_before_waiting() {
    let mut f = Fixture::new();
    f.run.backoff_ms = 30_000;
    f.run.pipeline_started_at = std::time::Instant::now() - Duration::from_secs(30);
    assert_eq!(f.recover(crash()).await, Duration::from_secs(2));
    assert_eq!(f.run.backoff_ms, 4000);
}

#[tokio::test(start_paused = true)]
async fn t1682_clean_stop_neither_waits_nor_grows_delay() {
    let mut f = Fixture::new();
    let changes = SessionChanges {
        display_dropped: true,
        ..crash()
    };
    assert_eq!(f.recover(changes).await, Duration::ZERO);
    assert_eq!(f.run.backoff_ms, 2000);
    f.run.pipeline_started_at = std::time::Instant::now() - Duration::from_secs(30);
    f.run.backoff_ms = 30_000;
    assert_eq!(f.recover(changes).await, Duration::ZERO);
    assert_eq!(f.run.backoff_ms, 2000);
}

#[tokio::test(start_paused = true)]
async fn t1682_pending_native_retirement_prevents_retry_and_backoff_growth() {
    let mut f = Fixture::new();
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let slot = pool.reserve().unwrap();
    let original = slot.completion();
    f.manager.encoder.pending = Some(original.clone());
    assert!(f
        .manager
        .recover_encoder_session(crash(), &mut f.run)
        .await
        .is_err());
    assert_eq!(f.run.backoff_ms, 2000);
    assert_eq!(f.run.encoder_mode, Some((1280, 800)));
    assert!(!original.is_complete());
    drop(slot);
    assert!(original.is_complete());
    assert_eq!(f.recover(crash()).await, Duration::from_secs(2));
}

#[tokio::test(start_paused = true)]
async fn t1682_retry_sleep_is_cancelled_by_display_loss_or_shutdown() {
    for shutdown in [false, true] {
        let mut f = Fixture::new();
        let mut recovery = Box::pin(f.manager.recover_encoder_session(crash(), &mut f.run));
        assert!(futures_util::poll!(&mut recovery).is_pending());
        tokio::time::advance(Duration::from_millis(250)).await;
        if shutdown {
            f.stop.send(true).unwrap();
        } else {
            f.display.send(false).unwrap();
        }
        assert!(futures_util::poll!(&mut recovery).is_ready());
        drop(recovery);
        assert_eq!(f.run.backoff_ms, 4000);
    }
}
