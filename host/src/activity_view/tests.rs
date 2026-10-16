//! T1043: original source wake/loss/fallback, independent of native authority.
use super::*;
use blent_config::host_activity::*;
use tokio::time::{timeout, Duration};

fn publication(power: bool) -> Publication {
    let capabilities = Capabilities {
        power,
        ..Default::default()
    };
    let mut feed = Feed::new(1, capabilities, Settings::default()).unwrap();
    if power {
        feed.observe(Event {
            epoch: 1,
            revision: 1,
            state: HostState {
                power: Some(Power::Running),
                ..Default::default()
            },
        })
        .unwrap();
    }
    Publication::Observed {
        capabilities,
        snapshot: feed.snapshot(true, false),
    }
}
#[tokio::test]
async fn t1043_watch_notice_is_armed_before_publication_and_first_poll_closure() {
    let (owner, source) = watch::channel(Publication::Pending);
    let view = View::new(ActivityTarget::Display, Some(source));
    assert!(view.begin().is_none());
    let notice = view.changed();
    owner.send_replace(publication(true));
    timeout(Duration::from_millis(20), notice).await.unwrap();
    let original = view.begin().unwrap();
    assert!(view.permits(original));
    let closed = view.changed();
    drop(owner);
    assert!(!view.permits(original));
    assert!(view.begin().is_none());
    timeout(Duration::from_millis(20), closed).await.unwrap();
    assert!(timeout(Duration::from_millis(10), view.changed())
        .await
        .is_err());
}
#[tokio::test]
async fn t1043_unsupported_original_source_and_absent_composition_stay_idle_and_usable() {
    let absent = View::new(ActivityTarget::Auxiliary, None);
    assert!(absent.permits(absent.begin().unwrap()));
    assert!(timeout(Duration::from_millis(10), absent.changed())
        .await
        .is_err());
    let (owner, source) = watch::channel(publication(false));
    let view = View::new(ActivityTarget::Display, Some(source));
    let original = view.begin().unwrap();
    let closed = view.changed();
    drop(owner);
    timeout(Duration::from_millis(20), closed).await.unwrap();
    assert!(view.permits(original));
    assert!(view.begin().is_some());
    assert!(timeout(Duration::from_millis(10), view.changed())
        .await
        .is_err());
}
#[tokio::test]
async fn t1043_replacement_view_never_changes_the_original_publication_source() {
    let (old, source) = watch::channel(publication(true));
    let view = View::new(ActivityTarget::Display, Some(source));
    let original = view.begin().unwrap();
    let (_new, source) = watch::channel(publication(true));
    let replacement = View::new(ActivityTarget::Display, Some(source));
    let mut lost = publication(true);
    if let Publication::Observed { snapshot, .. } = &mut lost {
        snapshot.source_lost = true;
    }
    old.send_replace(lost);
    assert!(!view.permits(original));
    assert!(replacement.permits(replacement.begin().unwrap()));
}
