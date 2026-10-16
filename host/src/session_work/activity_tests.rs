//! T1027: portable session construction forwards the same original publication.
use super::*;
use blent_config::host_activity::{
    ActivityTarget, Capabilities, Event, Feed, HostState, Power, Publication,
};
use tokio::sync::oneshot;

struct ActivityCapture(oneshot::Sender<Option<watch::Receiver<Publication>>>);
impl CaptureBackend for ActivityCapture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        self.0.send(context.activity).unwrap();
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                context.stop.wait_for(|stop| *stop).await.unwrap();
                Ok(())
            }),
        }
    }
}
async fn session(
    activity: Option<watch::Receiver<Publication>>,
) -> (
    Runtime,
    crate::session_network::RetainedListeners,
    Option<watch::Receiver<Publication>>,
    watch::Sender<bool>,
) {
    let (ready, started) = oneshot::channel();
    let (stop, stopped) = watch::channel(false);
    let (runtime, listeners) = prepared(Box::new(ActivityCapture(ready)), (0, 0))
        .with_activity(activity)
        .start_exported(stopped)
        .await
        .unwrap();
    (runtime, listeners, started.await.unwrap(), stop)
}
#[tokio::test]
async fn t1027_sessions_share_original_activity_capabilities_and_stop_fences() {
    let (published, observer) = watch::channel(Publication::Pending);
    let mut sessions = Vec::new();
    for _ in 0..3 {
        sessions.push(session(Some(observer.clone())).await);
    }
    let capabilities = Capabilities {
        power: true,
        lock: true,
        display: true,
    };
    let mut feed = Feed::new(7, capabilities, Default::default()).unwrap();
    feed.observe(Event {
        epoch: 7,
        revision: 1,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(false),
            display_on: Some(true),
        },
    })
    .unwrap();
    let started = feed.snapshot(true, false).demand.stamp;
    feed.observe(Event {
        epoch: 7,
        revision: 2,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(true),
            display_on: Some(false),
        },
    })
    .unwrap();
    feed.observe(Event {
        epoch: 7,
        revision: 3,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(false),
            display_on: Some(true),
        },
    })
    .unwrap();
    let latest = Publication::Observed {
        capabilities,
        snapshot: feed.snapshot(true, false),
    };
    published.send_replace(latest);
    let mut forwarded = 0;
    for (runtime, listeners, activity, _stop) in sessions {
        if let Some(activity) = activity {
            assert!(activity.same_channel(&observer));
            assert_eq!(*activity.borrow(), latest);
            let Publication::Observed {
                capabilities: seen,
                snapshot,
            } = *activity.borrow()
            else {
                unreachable!()
            };
            assert_eq!(seen, capabilities);
            assert!(snapshot.demand.display);
            assert!(snapshot.must_retire(ActivityTarget::Display, started));
            forwarded += 1;
        }
        runtime.stop().await;
        drop(listeners);
    }
    assert_eq!(
        forwarded, 3,
        "T1027 capture lost original activity subscription"
    );
}
#[tokio::test]
async fn t1027_missing_activity_source_remains_explicitly_unsupported() {
    let (runtime, listeners, activity, _stop) = session(None).await;
    assert!(activity.is_none());
    runtime.stop().await;
    drop(listeners);
}
#[tokio::test]
async fn t1027_session_receives_original_failure_stop_and_closed_source() {
    let (published, observer) = watch::channel(Publication::Pending);
    let (runtime, listeners, activity, _stop) = session(Some(observer)).await;
    let forwarded = activity.is_some();
    if let Some(mut activity) = activity {
        for publication in [Publication::Failed, Publication::Stopped] {
            published.send_replace(publication);
            activity.changed().await.unwrap();
            assert_eq!(*activity.borrow_and_update(), publication);
        }
        drop(published);
        assert!(activity.changed().await.is_err());
    }
    runtime.stop().await;
    drop(listeners);
    assert!(forwarded, "T1027 activity loss cannot reach capture");
}

#[test]
fn t1045_prepared_session_keeps_explicit_viewer_preferences() {
    let preferences = blent_config::host_activity::Settings {
        pause_on_lock: false, pause_on_display_off: false, pause_without_viewer: false,
    };
    let (ready, _started) = oneshot::channel();
    let session = prepared(Box::new(ActivityCapture(ready)), (0, 0))
        .with_activity_preferences(preferences);
    assert_eq!(session.activity_preferences, preferences, "T1045 saved preferences lost");
}

struct PreferenceCapture(oneshot::Sender<blent_config::host_activity::Settings>);
impl CaptureBackend for PreferenceCapture {
    fn start(self: Box<Self>, mut context: CaptureContext) -> CaptureWorkers {
        self.0.send(context.activity_preferences).unwrap();
        CaptureWorkers {
            auxiliary: vec![],
            capture: Box::pin(async move {
                context.stop.wait_for(|stop| *stop).await.unwrap();
                Ok(())
            }),
        }
    }
}
#[tokio::test]
async fn t1045_original_saved_preferences_reach_the_capture_boundary() {
    let preferences = blent_config::host_activity::Settings {
        pause_on_lock: false, pause_on_display_off: false, pause_without_viewer: false,
    };
    let (ready, started) = oneshot::channel();
    let (_stop, stopped) = watch::channel(false);
    let (runtime, listeners) = prepared(Box::new(PreferenceCapture(ready)), (0, 0))
        .with_activity_preferences(preferences).start_exported(stopped).await.unwrap();
    assert_eq!(started.await.unwrap(), preferences);
    runtime.stop().await;
    drop(listeners);
}
