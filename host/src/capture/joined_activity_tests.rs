//! T1009: real native display cleanup joined to reversible machine demand.
use super::*;

fn publish(
    feed: &mut Feed,
    published: &watch::Sender<Publication>,
    revision: u64,
    state: HostState,
) {
    feed.observe(Event {
        epoch: 1,
        revision,
        state,
    })
    .unwrap();
    published.send_replace(publication(feed));
}
fn running() -> HostState {
    HostState {
        power: Some(Power::Running),
        locked: Some(false),
        display_on: Some(true),
    }
}
async fn held_resume(
    fixture: &mut Fixture,
    feed: &mut Feed,
    published: &watch::Sender<Publication>,
    state: HostState,
    turn: u64,
    first: crate::media::VideoPacket,
) -> crate::media::VideoPacket {
    let now = Instant::now() + Duration::from_secs(100 * turn);
    let root = crate::runtime::runtime_dir().unwrap();
    let _ = std::fs::remove_file(root.join("release-pause"));
    let _ = std::fs::remove_file(root.join("pause-requested"));
    std::fs::write(root.join("hold-pause"), b"hold original native writer ACK").unwrap();
    let count = fixture.count.load(Ordering::SeqCst);
    publish(feed, published, 2 + 2 * turn, state);
    until(|| {
        fixture.connections.poll(&mut fixture.slots, now).unwrap();
        fixture.connections.status().is_empty() && !first.generation.load(Ordering::Acquire)
    })
    .await;
    until(|| root.join("pause-requested").exists()).await;
    publish(feed, published, 3 + 2 * turn, running());
    for _ in 0..32 {
        fixture
            .connections
            .poll(&mut fixture.slots, now + Duration::from_secs(60))
            .unwrap();
    }
    assert_eq!(
        fixture.count.load(Ordering::SeqCst),
        count,
        "T1009 resume substituted for original native ACK"
    );
    assert!(fixture.connections.status().is_empty());
    let ticket = fixture
        .slots
        .ticket(&blent::display_slots::Owner::Peer(peer(2)))
        .unwrap();
    assert_eq!(fixture.slots.closing(ticket), Some(false));
    while fixture.received.try_recv().is_ok() {}
    std::fs::write(
        root.join("release-pause"),
        b"original native writer may ACK",
    )
    .unwrap();
    fixture.active(now + Duration::from_secs(61)).await;
    let fresh = packet(&mut fixture.received).await;
    assert_eq!(fixture.count.load(Ordering::SeqCst), count + 1);
    assert!(fresh.is_idr && fresh.codec_config.is_some());
    assert!(!first.generation.load(Ordering::Acquire));
    assert!(fresh.generation.load(Ordering::Acquire));
    assert!(!Arc::ptr_eq(&first.generation, &fresh.generation));
    fresh
}
#[tokio::test]
async fn t1009_joined_display_off_suspend_unavailable_resume_wait_for_original_native_ack() {
    if ownership_tests::isolated("capture::activity_tests::registered::joined::t1009_joined_display_off_suspend_unavailable_resume_wait_for_original_native_ack") { return; }
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    let (published, source) = watch::channel(publication(&feed));
    let mut fixture = Fixture::new(source).await;
    let outcome = futures_util::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        fixture.active(Instant::now()).await;
        let mut first = packet(&mut fixture.received).await;
        for (turn, state) in [
            HostState {
                display_on: Some(false),
                ..running()
            },
            HostState {
                power: Some(Power::Suspending),
                ..running()
            },
            HostState {
                power: Some(Power::Unavailable),
                ..running()
            },
        ]
        .into_iter()
        .enumerate()
        {
            first = held_resume(
                &mut fixture,
                &mut feed,
                &published,
                state,
                turn as u64,
                first,
            )
            .await;
        }
    }))
    .await;
    let outcome = fixture.finish(outcome).await;
    if let Err(failure) = outcome {
        std::panic::resume_unwind(failure);
    }
}
#[tokio::test]
async fn t1009_coalesced_resume_and_original_authority_loss_cannot_replay_native_pixels() {
    if ownership_tests::isolated("capture::activity_tests::registered::joined::t1009_coalesced_resume_and_original_authority_loss_cannot_replay_native_pixels") { return; }
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    let (published, source) = watch::channel(publication(&feed));
    let mut fixture = Fixture::new(source).await;
    let outcome = futures_util::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(async {
        let now = Instant::now();
        fixture.active(now).await;
        let first = packet(&mut fixture.received).await;
        publish(
            &mut feed,
            &published,
            2,
            HostState {
                power: Some(Power::Suspending),
                ..running()
            },
        );
        publish(&mut feed, &published, 3, running());
        fixture.paused(&first, now).await;
        fixture.active(now + Duration::from_secs(2)).await;
        let fresh = packet(&mut fixture.received).await;
        assert!(!first.generation.load(Ordering::Acquire));
        assert!(fresh.is_idr && fresh.codec_config.is_some());
        fixture.source_closed.store(true, Ordering::SeqCst);
        fixture
            .connections
            .poll(&mut fixture.slots, now + Duration::from_secs(3))
            .unwrap();
        assert!(fixture.connections.status().is_empty());
        assert_eq!(fixture.count.load(Ordering::SeqCst), 2);
        fresh.generation
    }))
    .await;
    let outcome = fixture.finish(outcome).await;
    match outcome {
        Ok(generation) => assert!(!generation.load(Ordering::Acquire)),
        Err(failure) => std::panic::resume_unwind(failure),
    }
}

#[tokio::test]
async fn t1009_assertion_unwind_cannot_return_before_original_native_fixture_retirement() {
    if ownership_tests::isolated("capture::activity_tests::registered::joined::t1009_assertion_unwind_cannot_return_before_original_native_fixture_retirement") { return; }
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    let (_published, source) = watch::channel(publication(&feed));
    let mut fixture = Fixture::new(source).await;
    fixture.active(Instant::now()).await;
    let first = packet(&mut fixture.received).await;
    let native = fixture.retirement.clone();
    assert!(!native.is_complete());
    let failure = std::panic::catch_unwind(|| panic!("T1009 deliberate assertion failure"));
    let result = fixture.finish(failure).await;
    let settled_at_return = native.is_complete();
    // Even the faulty control retains its original cleanup sponsor. Await it
    // before reporting a failed oracle, so the private root stays owned.
    until(|| native.is_complete()).await;
    assert!(result.is_err());
    assert!(!first.generation.load(Ordering::Acquire));
    assert!(
        settled_at_return,
        "T1009 assertion unwinding returned before original native settlement"
    );
}
