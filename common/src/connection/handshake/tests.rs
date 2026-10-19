use super::*;
#[test]
fn t1104_tls_timeout_defaults_precision_and_seeded_boundaries() {
    assert_eq!(Timeout::default().duration(), Duration::from_secs(10));
    for duration in [
        Duration::from_nanos(1),
        Duration::from_millis(1),
        Duration::from_millis(37),
        Duration::from_secs(60),
    ] {
        assert_eq!(Timeout::new(duration).unwrap().duration(), duration);
    }
    for duration in [
        Duration::ZERO,
        Duration::from_secs(60) + Duration::from_nanos(1),
        Duration::MAX,
    ] {
        assert!(Timeout::new(duration).is_err());
    }
    let mut seed = 1104u64;
    for _ in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let milliseconds = seed % 120_001;
        assert_eq!(
            Timeout::new(Duration::from_millis(milliseconds)).is_ok(),
            (1..=60_000).contains(&milliseconds)
        );
    }
}
