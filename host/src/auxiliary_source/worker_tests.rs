//! T1081: independent native join oracle, with cleanup even for a lying control.
use super::*;
use std::time::Duration;

#[test]
fn t1081_retiring_thread_requires_actual_join_before_completion() {
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let proof = admission.completion();
    let (release, gate) = std::sync::mpsc::channel();
    let original = std::thread::spawn(move || {
        let _ = gate.recv();
    });
    let mut retiring = Retiring {
        thread: Thread::new(original),
        source: None,
        cleanup: None,
        interval: Duration::from_millis(1),
    };
    let withheld = !retiring.poll_complete();
    release.send(()).unwrap();
    // Cleanup uses the exact join primitive independently of the result under
    // test, so a deliberately premature completion cannot detach this thread.
    while !retiring.thread.poll_complete() {
        std::thread::sleep(Duration::from_millis(1));
    }
    admission.retain(retiring);
    while !proof.is_complete() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(withheld, "T1081 unjoined original was reported complete");
}

#[tokio::test]
async fn t1081_joined_thread_does_not_replace_independent_native_retirement() {
    use crate::auxiliary_source::tests::fixture::{Factory, Mode};
    let factory = Factory::new(Mode::Ready);
    let release = factory.hold();
    let source = Arc::new(
        Following::start(
            factory.clone(),
            Settings {
                interval: Duration::from_millis(1),
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let mut updates = source.subscribe();
    updates
        .wait_for(|value| matches!(value, Publication::Observed { .. }))
        .await
        .unwrap();
    let original = factory.original(0);
    let proof = original.completion.lock().unwrap().clone().unwrap();
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let mut thread = Thread::new(std::thread::spawn(|| ()));
    while !thread.poll_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    admission.retain(Retiring {
        thread,
        source: Some(source),
        cleanup: None,
        interval: Duration::from_millis(1),
    });
    while !original.stopped.load(std::sync::atomic::Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(!proof.is_complete());
    assert_eq!(
        pool.active(),
        1,
        "T1081 joined thread lost native ownership"
    );
    release.send(()).unwrap();
    while pool.active() != 0 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(proof.is_complete());
    assert!(original.dropped.load(std::sync::atomic::Ordering::Acquire));
}
