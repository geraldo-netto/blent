//! T1177: exhausted viewer admission must not acquire any receiver or grant.
use super::*;
use crate::video_queue;
use futures_util::FutureExt;

struct Capability(bool);
impl LivenessCapability for Capability {
    fn supported(&self) -> bool {
        self.0
    }
    fn changed(&self) -> Signal<'static> {
        Box::pin(std::future::pending())
    }
}

#[test]
fn t1177_denial_keeps_original_presence_capability_and_subscription() {
    let (video, raw) = video_queue::channel(8, Default::default());
    drop(raw);
    video.viewer_epoch().store(u64::MAX - 1, Ordering::Release);
    let original = video.subscribe_negotiated(Arc::new(Capability(true)));
    let before = video.viewers.presence().unwrap();
    assert!(video.viewers.liveness_ready());
    let notice = video.viewers.changed();
    let capability = Arc::new(Capability(false));
    for _ in 0..3 {
        let denied = video.subscribe_negotiated(capability.clone());
        assert_eq!(video.receiver_count(), 1, "T1177: denied receiver admitted");
        assert_eq!(video.viewers.members.lock().unwrap().entries.len(), 1);
        assert_eq!(video.viewers.epoch(), u64::MAX);
        assert!(video.viewers.presence().unwrap().continues(&before));
        assert!(video.viewers.liveness_ready());
        assert_eq!(Arc::strong_count(&capability), 1);
        drop(denied);
    }
    assert!(
        notice.now_or_never().is_none(),
        "T1177: denial changed membership"
    );
    let retained_observation = video.viewers.changed();
    drop(original);
    assert_eq!(video.receiver_count(), 0);
    assert!(video.viewers.members.lock().unwrap().entries.is_empty());
    assert!(!video.viewers.presence().unwrap().present());
    assert!(!video.viewers.liveness_ready());
    assert!(retained_observation.now_or_never().is_some());
}

#[test]
fn t1177_terminal_empty_registry_cannot_restart_continuity() {
    let (video, raw) = video_queue::channel(8, Default::default());
    let prior = video.viewers.presence().unwrap();
    drop(raw);
    video.viewer_epoch().store(u64::MAX, Ordering::Release);
    let notice = video.viewers.changed();
    let denied = video.subscribe();
    assert_eq!(
        video.receiver_count(),
        0,
        "T1177: terminal registry restarted"
    );
    assert!(!video.viewers.presence().unwrap().continues(&prior));
    assert!(video.viewers.members.lock().unwrap().entries.is_empty());
    assert_eq!(video.viewers.epoch(), u64::MAX);
    drop(denied);
    assert!(notice.now_or_never().is_none());
    assert_eq!(Arc::strong_count(&video.viewers), 1);
}

#[test]
fn t1177_concurrent_final_admission_reserves_only_one_original() {
    let (video, raw) = video_queue::channel(8, Default::default());
    drop(raw);
    video.viewer_epoch().store(u64::MAX - 1, Ordering::Release);
    let gate = Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let (video, gate) = (video.clone(), gate.clone());
            std::thread::spawn(move || {
                gate.wait();
                video.subscribe()
            })
        })
        .collect();
    let originals: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        video.receiver_count(),
        1,
        "T1177: final reservation had multiple winners"
    );
    assert_eq!(video.viewers.members.lock().unwrap().entries.len(), 1);
    assert_eq!(video.viewers.epoch(), u64::MAX);
    drop(originals);
    assert_eq!(video.receiver_count(), 0);
    assert!(video.viewers.members.lock().unwrap().entries.is_empty());
    assert_eq!(Arc::strong_count(&video.viewers), 1);
}

#[test]
fn t1177_bounded_wide_oracle_rejects_without_receivers_or_epoch_reuse() {
    for seed in (0..=16).chain(u64::MAX - 16..=u64::MAX) {
        check_seed(seed);
    }
}

fn check_seed(seed: u64) {
    let (video, raw) = video_queue::channel(8, Default::default());
    drop(raw);
    video.viewer_epoch().store(seed, Ordering::Release);
    let mut expected = u128::from(seed);
    for _ in 0..20 {
        let receiver = video.subscribe();
        let admitted = usize::from(expected < u128::from(u64::MAX));
        expected += admitted as u128;
        assert_eq!(
            video.receiver_count(),
            admitted,
            "T1177: wide admission oracle"
        );
        assert_eq!(u128::from(video.viewers.epoch()), expected);
        assert_eq!(
            video.viewers.members.lock().unwrap().entries.len(),
            admitted
        );
        drop(receiver);
        assert_eq!(video.receiver_count(), 0);
        assert!(video.viewers.members.lock().unwrap().entries.is_empty());
    }
    assert_eq!(Arc::strong_count(&video.viewers), 1);
}
