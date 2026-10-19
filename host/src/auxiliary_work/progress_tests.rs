//! T1085: phase/report scopes are independent from original native cleanup.
use super::*;
use crate::{audio_control, camera_control, report_scope};
use blent_config::{
    audio::{AudioState, AudioStatus},
    camera::CameraState,
};

struct Reports {
    camera: camera_control::Report,
    audio: audio_control::Report,
    scope: report_scope::Owner,
    phases: Mutex<Vec<Phase>>,
}
impl Reports {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            camera: camera_control::Report::new(
                watch::channel(CameraState::Stopped).0,
                watch::channel(None).0,
            ),
            audio: audio_control::Report::new(watch::channel(AudioStatus::default()).0),
            scope: Default::default(),
            phases: Mutex::new(Vec::new()),
        })
    }
    fn progress(&self, phase: Phase) {
        match phase {
            Phase::Starting => self.scope.begin(),
            _ => self.scope.revoke(),
        }
        self.camera.activity(phase);
        self.audio.activity(phase);
        let mut phases = self.phases.lock().unwrap();
        assert!(phases.len() < 32);
        phases.push(phase);
    }
    fn start(
        &self,
        native: &Native,
        stop: watch::Receiver<bool>,
    ) -> impl Future<Output = Result<()>> + Send + 'static {
        let permit = self.scope.permit().unwrap();
        let camera = self.camera.scoped(permit.clone());
        let audio = self.audio.scoped(permit);
        let work = native.start(stop);
        async move {
            camera.update(CameraState::Streaming);
            audio.update(AudioState::Streaming, "live");
            let result = work.await;
            // Last callbacks may run before native retirement. Cutoff must seal
            // them; a retained clone never becomes current in a later attempt.
            camera.update(CameraState::Waiting);
            camera.publish_preview(None);
            audio.update(AudioState::Streaming, "late old callback");
            result
        }
    }
}
fn observed_work(
    source: &Source,
    native: &Arc<Native>,
    reports: &Arc<Reports>,
    stop: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<Result<()>> {
    let source = source.owner.subscribe();
    let native = native.clone();
    let reports = reports.clone();
    tokio::spawn(async move {
        run_observed(
            Some(source),
            stop,
            |stop| reports.start(&native, stop),
            || native.settle(),
            |phase| reports.progress(phase),
        )
        .await
    })
}

#[tokio::test]
async fn t1085_paused_intent_keeps_stop_and_no_early_retirement_or_stale_reports() {
    let mut source = Source::new(true);
    source.emit(Power::Suspending, false, true);
    let native = Native::new(false);
    let reports = Reports::new();
    let (stop, stopped) = watch::channel(false);
    let work = observed_work(&source, &native, &reports, stopped);
    until(|| reports.camera.state() == CameraState::Paused).await;
    assert!(reports.camera.state().is_active());
    assert_eq!(reports.audio.status().state, AudioState::Paused);
    assert!(reports.audio.status().state.is_active());
    assert_eq!(native.count(), 0);
    source.emit(Power::Running, false, true);
    until(|| native.count() == 1).await;
    let old_permit = reports.scope.permit().unwrap();
    let old_camera = reports.camera.scoped(old_permit.clone());
    let old_audio = reports.audio.scoped(old_permit);
    source.emit(Power::Suspending, false, true);
    until(|| native.original(0).stopped.load(Ordering::Acquire)).await;
    assert_eq!(reports.camera.state(), CameraState::Stopping);
    assert_eq!(reports.audio.status().state, AudioState::Stopping);
    assert!(!native.original(0).settled.load(Ordering::Acquire));
    native.original(0).release.send_replace(true);
    until(|| reports.camera.state() == CameraState::Paused).await;
    source.emit(Power::Running, false, true);
    until(|| native.count() == 2).await;
    old_camera.update(CameraState::Failed("stale".into()));
    old_audio.update(AudioState::Failed, "stale");
    assert_eq!(reports.camera.state(), CameraState::Streaming);
    assert_eq!(reports.audio.status().state, AudioState::Streaming);
    stop.send_replace(true);
    until(|| native.original(1).stopped.load(Ordering::Acquire)).await;
    assert!(!work.is_finished());
    native.release_all();
    work.await.unwrap().unwrap();
    native.conserved();
    assert_eq!(reports.camera.state(), CameraState::Stopped);
    assert!(!reports.camera.state().is_active());
    assert!(!reports.audio.status().state.is_active());
}

#[tokio::test]
async fn t1085_stop_while_initially_paused_cancels_retained_intent_without_native_work() {
    let mut source = Source::new(true);
    source.emit(Power::Unavailable, true, false);
    let native = Native::new(true);
    let reports = Reports::new();
    let (stop, stopped) = watch::channel(false);
    let work = observed_work(&source, &native, &reports, stopped);
    until(|| reports.camera.state() == CameraState::Paused).await;
    stop.send_replace(true);
    work.await.unwrap().unwrap();
    source.emit(Power::Running, false, true);
    assert_eq!(native.count(), 0);
    assert_eq!(
        *reports.phases.lock().unwrap(),
        [Phase::Paused, Phase::Finished]
    );
}

#[tokio::test]
async fn t1085_reporter_panics_do_not_bypass_original_work_or_native_settlement() {
    for phase in [
        Phase::Paused,
        Phase::Starting,
        Phase::Retiring,
        Phase::Finished,
    ] {
        reporter_failure(phase).await;
    }
}
async fn reporter_failure(failed: Phase) {
    let mut source = Source::new(true);
    if failed == Phase::Paused {
        source.emit(Power::Suspending, false, true);
    }
    let native = Native::new(false);
    let (stop, stopped) = watch::channel(false);
    let owner = native.clone();
    let source = source.owner.subscribe();
    let work = tokio::spawn(async move {
        run_observed(
            Some(source),
            stopped,
            |stop| owner.start(stop),
            || owner.settle(),
            |phase| {
                assert_ne!(phase, failed, "T1085 reporter panic");
            },
        )
        .await
    });
    if matches!(failed, Phase::Retiring | Phase::Finished) {
        until(|| native.count() == 1).await;
        stop.send_replace(true);
        until(|| native.original(0).stopped.load(Ordering::Acquire)).await;
        assert!(
            !work.is_finished(),
            "T1085 reporter failure skipped native fence"
        );
        native.release_all();
    }
    assert!(work.await.unwrap().is_err());
    native.conserved();
}

#[tokio::test]
async fn t1085_natural_finish_and_reporter_failure_still_wait_for_native_settlement() {
    for failed in [false, true] {
        natural_finish(failed).await;
    }
}
async fn natural_finish(failed: bool) {
    let native = Native::new(false);
    let owner = native.clone();
    let reports = Reports::new();
    let output = reports.clone();
    let (_manual, stop) = watch::channel(false);
    let work = tokio::spawn(async move {
        run_observed(
            None,
            stop,
            |_| {
                let (_stop, stopped) = watch::channel(true);
                owner.start(stopped)
            },
            || owner.settle(),
            |phase| {
                output.progress(phase);
                assert!(
                    !(failed && phase == Phase::Retiring),
                    "T1085 natural reporter failure"
                );
            },
        )
        .await
    });
    until(|| reports.camera.state() == CameraState::Stopping).await;
    assert!(!work.is_finished());
    native.release_all();
    assert_eq!(work.await.unwrap().is_err(), failed);
    native.conserved();
}

#[tokio::test]
async fn t1085_scoped_preview_release_and_native_proof_ownership_are_independent() {
    use blent_config::{
        camera::{CameraPreview, Lens},
        retirement::Pool,
    };
    let reports = Reports::new();
    reports.progress(Phase::Starting);
    let permit = reports.scope.permit().unwrap();
    let camera = reports.camera.scoped(permit.clone());
    let audio = reports.audio.scoped(permit);
    let frame = Arc::new(CameraPreview::new(Lens::Front, [1, 1], vec![0; 4]).unwrap());
    let old = Arc::downgrade(&frame);
    camera.publish_preview(Some(frame.clone()));
    assert!(reports.camera.preview().is_some());
    let pool = Pool::new(1).unwrap();
    let admitted = pool.reserve().unwrap();
    audio.retain(admitted.completion()).unwrap();
    reports.progress(Phase::Retiring);
    assert!(reports.camera.preview().is_none());
    camera.publish_preview(Some(frame));
    assert!(
        old.upgrade().is_none(),
        "T1085 revoked preview retained stale frame"
    );
    assert!(
        !reports.audio.native_retired(),
        "T1085 report cutoff released native proof"
    );
    drop(admitted);
    reports.audio.wait_native().await;
    assert!(reports.audio.native_retired());
    reports.progress(Phase::Finished);
}
