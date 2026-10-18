//! T1045: local viewer demand is independent of machine capability and ACKs.
use super::*;

#[tokio::test]
async fn t1050_registration_pause_fences_current_and_future_work_until_exact_receipts() {
    let (control, participants) = crate::display_quiescence::channel(1).unwrap();
    let legacy = View::new(None, None, false);
    let unmanaged = legacy.begin().unwrap();
    let gated = legacy.with_gate(participants[0].source());
    assert!(!gated.permits(&unmanaged));
    let original = gated.begin().unwrap();
    assert!(gated.permits(&original));
    let notice = gated.changed();
    let fence = control.pause();
    assert!(gated.begin().is_none());
    assert!(!gated.permits(&original));
    participants[0].acknowledge(&participants[0].pending().unwrap());
    assert!(control.resume(&fence));
    notice.await;
    assert!(!gated.permits(&original), "T1050 coalesced pause revived old work");
    let renewed = gated.begin().unwrap();
    assert!(!View::new(None, None, false).permits(&renewed));
    drop(control);
    assert!(gated.begin().is_none());
    assert!(!gated.permits(&renewed));
}
use crate::{capture_contract::LivenessCapability, video_queue};
use blent_config::host_activity::*;
use tokio::time::{timeout, Duration};

fn video() -> video_queue::VideoSender {
    let (video, initial) = video_queue::channel(8, Default::default());
    drop(initial);
    video
}
fn view(video: &video_queue::VideoSender, pause: bool) -> View {
    View::new(None, Some(video.viewer_activity()), pause)
}
#[test]
fn t1045_no_viewer_denies_work_even_without_machine_support_or_ack() {
    let video = video();
    let work = view(&video, true);
    assert!(
        work.begin().is_none(),
        "T1045 empty session still creates work"
    );
    let receiver = video.subscribe();
    assert!(!video.viewer_activity().liveness_ready());
    let original = work.begin().unwrap();
    assert!(work.permits(&original));
    drop(receiver);
    assert!(!work.permits(&original));
    assert!(work.begin().is_none());
}
#[test]
fn t1045_coalesced_last_viewer_replacement_cannot_revive_original_work() {
    let video = video();
    let work = view(&video, true);
    let first = video.subscribe();
    let original = work.begin().unwrap();
    drop(first);
    let _replacement = video.subscribe();
    assert!(work.begin().is_some());
    assert!(
        !work.permits(&original),
        "T1045 coalesced viewer gap revived old work"
    );
}
#[test]
fn t1045_continuous_viewers_do_not_restart_work_for_late_joins_or_departures() {
    let video = video();
    let work = view(&video, true);
    let first = video.subscribe();
    let original = work.begin().unwrap();
    let second = video.subscribe();
    assert!(work.permits(&original));
    drop(first);
    assert!(work.permits(&original));
    drop(second);
    assert!(!work.permits(&original));
}
#[test]
fn t1045_disabled_preference_and_explicit_unsupported_presence_keep_fallback() {
    let video = video();
    let work = view(&video, false);
    let original = work.begin().unwrap();
    let receiver = video.subscribe();
    drop(receiver);
    assert!(work.permits(&original));
    let unsupported = View::new(None, None, true);
    assert!(unsupported.permits(&unsupported.begin().unwrap()));
    let activity: Arc<dyn ViewerActivity> = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let unsupported = View::new(None, Some(activity), true);
    assert!(unsupported.permits(&unsupported.begin().unwrap()));
}
#[tokio::test]
async fn t1045_viewer_notices_arm_before_first_poll_without_retaining_membership() {
    let video = video();
    let work = view(&video, true);
    let joined = work.changed();
    let receiver = video.subscribe();
    timeout(Duration::from_millis(30), joined).await.unwrap();
    let left = work.changed();
    drop(receiver);
    timeout(Duration::from_millis(30), left).await.unwrap();
    assert!(work.begin().is_none());
    assert_eq!(video.receiver_count(), 0);
    assert!(timeout(Duration::from_millis(20), work.changed())
        .await
        .is_err());
}
fn feed() -> Feed {
    let capabilities = Capabilities {
        power: true,
        lock: true,
        display: true,
    };
    let mut feed = Feed::new(1, capabilities, Settings::default()).unwrap();
    event(&mut feed, 1, false);
    feed
}
fn event(feed: &mut Feed, revision: u64, locked: bool) {
    feed.observe(Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(locked),
            display_on: Some(true),
        },
    })
    .unwrap();
}
fn publication(feed: &Feed) -> Publication {
    Publication::Observed {
        capabilities: Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        snapshot: feed.snapshot(true, false),
    }
}
#[tokio::test]
async fn t1045_original_machine_fences_loss_and_pending_still_deny_with_viewers() {
    let video = video();
    let _receiver = video.subscribe();
    let mut feed = feed();
    let (publish, source) = watch::channel(publication(&feed));
    let work = View::new(Some(source), Some(video.viewer_activity()), true);
    let original = work.begin().unwrap();
    let notice = work.changed();
    event(&mut feed, 2, true);
    publish.send_replace(publication(&feed));
    event(&mut feed, 3, false);
    publish.send_replace(publication(&feed));
    timeout(Duration::from_millis(30), notice).await.unwrap();
    assert!(!work.permits(&original));
    for invalid in [
        Publication::Pending,
        Publication::Failed,
        Publication::Stopped,
    ] {
        publish.send_replace(invalid);
        assert!(work.begin().is_none());
    }
    publish.send_replace(publication(&feed));
    let original = work.begin().unwrap();
    let closed = work.changed();
    drop(publish);
    timeout(Duration::from_millis(30), closed).await.unwrap();
    assert!(!work.permits(&original));
    assert!(work.begin().is_none());
}
#[test]
fn t1045_presence_continuity_bounds_and_foreign_originals_are_safe() {
    let one = Arc::new(());
    let original = ViewerPresence::new(1, one.clone());
    for count in [0, 1, 16, usize::MAX] {
        let current = ViewerPresence::new(count, one.clone());
        assert_eq!(current.present(), count != 0);
        assert_eq!(current.continues(&original), count != 0);
        assert!(!current.continues(&ViewerPresence::new(count, Arc::new(()))));
        assert!(!current.continues(&ViewerPresence::new(0, one.clone())));
    }
}
struct Grant;
impl LivenessCapability for Grant {
    fn supported(&self) -> bool {
        true
    }
    fn changed(&self) -> Signal<'static> {
        Box::pin(std::future::pending())
    }
}
#[test]
fn t1045_acknowledgement_is_not_a_viewer_or_capture_authority() {
    let video = video();
    let work = view(&video, true);
    let source: Arc<dyn LivenessCapability> = Arc::new(Grant);
    assert!(source.supported());
    assert!(work.begin().is_none());
    let receiver = video.subscribe_negotiated(source);
    assert!(video.viewer_activity().liveness_ready());
    assert!(work.begin().is_some());
    drop(receiver);
    assert!(work.begin().is_none());
}

#[tokio::test]
async fn t1045_unsupported_notices_wait_without_idle_polling() {
    let absent = View::new(None, None, true);
    assert!(timeout(Duration::from_millis(20), absent.changed())
        .await
        .is_err());
    let unsupported: Arc<dyn ViewerActivity> = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let absent = View::new(None, Some(unsupported), true);
    assert!(timeout(Duration::from_millis(20), absent.changed())
        .await
        .is_err());
}
