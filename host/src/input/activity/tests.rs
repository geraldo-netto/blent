use super::*;
use blent_config::host_activity::*;
use std::time::Duration;

fn ready(feed: &mut Feed, revision: u64, locked: bool) -> Publication {
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
async fn t1056_original_notice_captures_publication_before_first_poll_and_coalesced_stops() {
    let (published, source) = watch::channel(Publication::Pending);
    let mut observer = Observer::new(Some(source));
    assert!(!observer.update().unwrap().unwrap().active());
    let wake = observer.changed();
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    published.send_replace(ready(&mut feed, 1, false));
    tokio::time::timeout(Duration::from_secs(1), wake)
        .await
        .unwrap();
    assert!(observer.update().unwrap().unwrap().active());
    published.send_replace(ready(&mut feed, 2, true));
    published.send_replace(ready(&mut feed, 3, false));
    let signal = observer.update().unwrap().unwrap();
    assert!(signal.active());
    assert_eq!(signal.stopped(), 1);
    assert!(observer.update().unwrap().is_none());
    let Message::Text(text) = message(signal).unwrap() else {
        panic!("T1056 expected bounded activity JSON")
    };
    assert!(text.len() < 256);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["status"], "display_activity");
    assert_eq!(
        serde_json::from_value::<Signal>(value["display_activity"].clone()).unwrap(),
        signal
    );
}
#[tokio::test]
async fn t1056_absent_and_closed_sources_cannot_spin_or_announce_unsupported_work() {
    let mut observer = Observer::new(None);
    assert!(observer.update().unwrap().is_none());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), observer.changed())
            .await
            .is_err()
    );
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    let (published, source) = watch::channel(ready(&mut feed, 1, false));
    let mut observer = Observer::new(Some(source));
    assert!(observer.update().unwrap().unwrap().active());
    let wake = observer.changed();
    drop(published);
    tokio::time::timeout(Duration::from_secs(1), wake)
        .await
        .unwrap();
    let signal = observer.update().unwrap().unwrap();
    assert!(!signal.active());
    assert_eq!(signal.stopped(), 1);
    assert!(observer.update().unwrap().is_none());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), observer.changed())
            .await
            .is_err()
    );
}
