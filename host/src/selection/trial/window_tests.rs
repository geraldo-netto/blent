//! T1491: bounded sample eviction must not shorten the trial's six-second budget.
use super::*;

#[tokio::test(start_paused = true)]
async fn t1491_trial_can_recover_after_more_than_256_render_acknowledgements() {
    for previous in [0, 20] {
        recover(previous).await;
    }
}

async fn recover(previous: u32) {
    let tracker = LatencyTracker::new();
    let key = Key {
        format: (640, 480, 60, 20_000, 18),
        epoch: 1,
        decoders: None,
    };
    let evidence = tracker.encoder_started("libx264", key.format);
    for _ in 0..previous {
        let sequence = tracker.next_sequence();
        tracker.on_encoded_for(sequence, &evidence);
        tracker.on_rendered(sequence, 100);
    }
    let worker = tokio::spawn({
        let tracker = tracker.clone();
        let key = key.clone();
        async move { observe(&tracker, &key, "libx264", None, 0).await }
    });
    tokio::task::yield_now().await;
    for frame in 1..=330 {
        tokio::time::advance(Duration::from_micros(16_666)).await;
        let sequence = tracker.next_sequence();
        tracker.on_encoded_for(sequence, &evidence);
        if !(5..=34).contains(&frame) {
            tracker.on_rendered(sequence, 100);
        }
        tokio::task::yield_now().await;
        if frame == 286 {
            assert!(
                !worker.is_finished(),
                "T1491 early loss has not reached 90% delivery"
            );
        }
    }
    let result = worker
        .await
        .unwrap()
        .expect("T1491 sample eviction truncated the six-second recovery window");
    assert_eq!(result.startup_us, 16_666);
    assert!((12..=256).contains(&result.samples));
    assert!(result.delivery_permille >= 900);
    assert!((59.0..=61.0).contains(&result.ack_fps));
    assert_eq!(evidence.samples_after(0).len(), 256);
}

fn sample(ordinal: u64, at: tokio::time::Instant) -> RenderSample {
    RenderSample {
        sequence: 0,
        ordinal,
        output: ordinal,
        at,
        micros: 100,
    }
}

#[test]
fn t1491_startup_requires_exact_first_ack_and_resets_for_each_encoder_epoch() {
    let started = tokio::time::Instant::now();
    for floor in [0, 20, u64::MAX - 2] {
        let mut startup = Startup::default();
        assert_eq!(startup.observe(1, floor, &[], started), None);
        let missing = sample(floor + 2, started + Duration::from_secs(1));
        assert_eq!(startup.observe(1, floor, &[missing], started), None);
        let first = sample(floor + 1, started + Duration::from_millis(100));
        assert_eq!(startup.observe(1, floor, &[first], started), Some(100_000));
        let later = sample(floor + 2, started + Duration::from_secs(5));
        assert_eq!(
            startup.observe(1, floor, &[later.clone()], started),
            Some(100_000)
        );
        assert_eq!(startup.observe(2, floor, &[later], started), None);
        let replacement = sample(floor + 1, started + Duration::from_secs(2));
        assert_eq!(
            startup.observe(2, floor, &[replacement], started),
            Some(2_000_000)
        );
    }
    let mut startup = Startup::default();
    assert_eq!(
        startup.observe(1, u64::MAX, &[sample(u64::MAX, started)], started),
        None
    );
}
