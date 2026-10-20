//! T1115: original registry membership, current receipts and native-file effects.
use super::*;
use crate::video_queue;
use blent::capture_contract::LivenessCapability;
use tokio::sync::watch;

struct Grant(watch::Receiver<bool>);
impl LivenessCapability for Grant {
    fn supported(&self) -> bool {
        self.0.has_changed().is_ok() && *self.0.borrow()
    }
    fn changed(&self) -> blent::capture_contract::Signal<'static> {
        let mut original = self.0.clone();
        let open = original.has_changed().is_ok();
        original.borrow_and_update();
        Box::pin(async move {
            if open {
                let _ = original.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        })
    }
}

pub(super) struct Trial {
    handle: Option<Handle>,
    tracker: LatencyTracker,
    evidence: Arc<EncoderEvidence>,
    _activity: crate::latency::EncoderActivity,
    video: video_queue::VideoSender,
    receiver: Option<video_queue::VideoReceiver>,
    grant: Option<watch::Sender<bool>>,
    _root: tempfile::TempDir,
}
impl Trial {
    pub(super) async fn new() -> Self {
        Self::for_encoder("libx264").await
    }
    async fn for_encoder(name: &str) -> Self {
        let (grant, capability) = watch::channel(true);
        Self::with_capability(name, Arc::new(Grant(capability)), Some(grant)).await
    }
    pub(super) async fn with_capability(
        name: &str,
        capability: Arc<dyn LivenessCapability>,
        grant: Option<watch::Sender<bool>>,
    ) -> Self {
        let root = super::lease_tests::private_root();
        let tracker = LatencyTracker::new();
        let template = super::tests::evidence(&tracker);
        let evidence =
            tracker.encoder_started_with_decoder(name, template.format, template.decoder.clone());
        let activity = tracker.encoder_activity(evidence.clone());
        let (video, initial) = video_queue::channel(8, Default::default());
        drop(initial);
        let receiver = video.subscribe_negotiated(capability).unwrap();
        let lease = super::tests::lease(root.path());
        let subjects = subjects::Subjects::new(Mode::Unchanged, video.viewer_activity());
        let handle = finish_startup(spawn(
            Mode::Unchanged,
            lease.stop.clone(),
            move || Ok(lease),
            evidence.clone(),
            tracker.clone(),
            subjects,
            &Default::default(),
        ))
        .await
        .unwrap();
        Self {
            handle: Some(handle),
            tracker,
            evidence,
            _activity: activity,
            video,
            receiver: Some(receiver),
            grant,
            _root: root,
        }
    }
    pub(super) fn handle(&self) -> &Handle {
        self.handle.as_ref().unwrap()
    }
    async fn receipt(&self, sequence: u32, pts: i64) {
        self.handle().note(sequence, Some(pts), true);
        self.tracker.on_encoded_for(sequence, &self.evidence);
        tokio::time::advance(Duration::from_millis(10)).await;
        self.tracker.on_rendered_from(
            sequence,
            100,
            Some(&self.evidence.decoder.as_ref().unwrap().receipt()),
        );
        super::tests::native_until(|| {
            !self
                .handle()
                .timings
                .lock()
                .unwrap()
                .iter()
                .any(|frame| frame.sequence == sequence)
        })
        .await;
    }
    async fn interval(&self, expected: u32) {
        let expected = expected.to_string();
        super::tests::native_until(|| {
            self.handle()
                ._lease
                .contents()
                .ok()
                .is_some_and(|text| text.split_ascii_whitespace().last() == Some(expected.as_str()))
        })
        .await;
    }
    pub(super) async fn qualify(&self) {
        for sequence in 0..=16 {
            self.receipt(sequence, i64::from(sequence) * 200_000).await;
            tokio::time::advance(Duration::from_millis(190)).await;
        }
        self.interval(500).await;
        for sequence in 17..=32 {
            tokio::time::advance(Duration::from_millis(300)).await;
            self.receipt(sequence, 3_200_000 + i64::from(sequence - 16) * 500_000)
                .await;
            tokio::time::advance(Duration::from_millis(190)).await;
        }
        self.interval(1000).await;
        for sequence in 33..=48 {
            tokio::time::advance(Duration::from_millis(800)).await;
            self.receipt(sequence, 11_200_000 + i64::from(sequence - 32) * 1_000_000)
                .await;
            tokio::time::advance(Duration::from_millis(190)).await;
        }
        self.interval(1000).await;
    }
    pub(super) async fn cleared(&self) {
        super::tests::native_until(|| !self.handle()._lease.path.exists()).await;
    }
    pub(super) async fn finish(mut self) {
        let original = self.handle().completion();
        drop(self.handle.take());
        super::tests::idle_completed(&original).await;
    }
}

#[tokio::test(start_paused = true)]
async fn t1115_current_receipts_admit_one_second_then_liveness_loss_wakes_without_timer() {
    let mut trial = Trial::new().await;
    trial.qualify().await;
    drop(trial.grant.take());
    // No virtual time advance: the original capability's closure must wake it.
    trial.cleared().await;
    trial.finish().await;
}

#[tokio::test(start_paused = true)]
async fn t1115_coalesced_viewer_replacement_cannot_reuse_current_qualification() {
    let mut trial = Trial::new().await;
    trial.qualify().await;
    drop(trial.receiver.take());
    let (_grant, capability) = watch::channel(true);
    let _replacement = trial
        .video
        .subscribe_negotiated(Arc::new(Grant(capability))).unwrap();
    trial.cleared().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    assert!(!trial.handle()._lease.path.exists());
    trial.finish().await;
}

#[tokio::test(start_paused = true)]
async fn t1115_one_second_mode_restores_compatibility_when_original_receipts_stop() {
    let trial = Trial::new().await;
    trial.qualify().await;
    tokio::time::advance(Duration::from_millis(1501)).await;
    trial.cleared().await;
    trial.finish().await;
}

#[tokio::test(start_paused = true)]
async fn t1115_unqualified_backend_cannot_enter_native_idle_trial() {
    let trial = Trial::for_encoder("h264_nvenc").await;
    super::tests::native_until(|| trial.handle().task.is_finished()).await;
    assert!(!trial.handle()._lease.path.exists());
    trial.finish().await;
}

#[tokio::test(start_paused = true)]
async fn t1115_missing_original_packet_timing_revokes_one_second_trial() {
    let trial = Trial::new().await;
    trial.qualify().await;
    trial.tracker.on_encoded_for(49, &trial.evidence);
    trial.tracker.on_rendered_from(
        49,
        100,
        Some(&trial.evidence.decoder.as_ref().unwrap().receipt()),
    );
    trial.cleared().await;
    trial.finish().await;
}
