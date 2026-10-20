//! T1201: exercise the same timestamp store used by the maintained probe.
#[path = "../examples/camera-feedback-probe/samples.rs"]
mod samples;
use samples::Samples;
use std::time::Duration;

#[test]
fn t1201_probe_capacity_is_admission_not_only_initial_reservation() {
    let mut samples = Samples::new(2).unwrap();
    samples.record(Duration::from_millis(1)).unwrap();
    samples.record(Duration::from_millis(2)).unwrap();
    assert!(
        samples.record(Duration::from_millis(3)).is_err(),
        "T1201 sample bound ignored"
    );
    assert_eq!(samples.count(), 2);
    assert_eq!(samples.first(), Some(1));
    assert_eq!(samples.timestamps(), [1, 2]);
}

#[test]
fn t1201_invalid_sample_capacity_rejects_before_observation() {
    assert!(
        Samples::new(0).is_err(),
        "T1201 zero sample capacity admitted"
    );
    assert!(
        Samples::new(1_048_577).is_err(),
        "T1201 oversized sample capacity admitted"
    );
}

#[test]
fn t1201_bounded_timestamp_admission_matches_independent_count_model() {
    for capacity in 1..=64 {
        let mut samples = Samples::new(capacity).unwrap();
        for attempt in 0..=capacity + 8 {
            let admitted = attempt < capacity;
            assert_eq!(
                samples
                    .record(Duration::from_millis(attempt as u64))
                    .is_ok(),
                admitted
            );
            assert_eq!(samples.count(), (attempt + 1).min(capacity));
        }
        assert_eq!(
            samples.timestamps(),
            (0..capacity as u64).collect::<Vec<_>>()
        );
    }
}

#[test]
fn t1201_invalid_duration_and_capacity_leave_observations_unchanged() {
    for capacity in [0, samples::MAX_CAPACITY + 1, usize::MAX] {
        assert!(Samples::new(capacity).is_err());
    }
    let mut samples = Samples::new(3).unwrap();
    assert_eq!(samples.count(), 0);
    assert_eq!(samples.first(), None);
    assert!(samples.record(Duration::MAX).is_err());
    assert!(samples.timestamps().is_empty());
    samples.record(Duration::from_millis(u64::MAX)).unwrap();
    assert_eq!(samples.first(), Some(u64::MAX));
    samples.record(Duration::from_millis(0)).unwrap();
    assert_eq!(samples.timestamps(), [u64::MAX, 0]);
}

#[test]
fn t1201_supported_capacity_endpoints_are_funded_and_json_safe() {
    for capacity in [1, samples::DEFAULT_CAPACITY, samples::MAX_CAPACITY] {
        let mut samples = Samples::new(capacity).unwrap();
        samples.record(Duration::from_millis(u64::MAX)).unwrap();
        assert_eq!(
            serde_json::to_value(samples.timestamps()).unwrap(),
            serde_json::json!([u64::MAX])
        );
    }
}
