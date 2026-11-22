//! Bounded observation cadence, independent of native readiness/ownership.
//! Three durations belong to one control loop. No resource or borrowed view is
//! retained; waiting never proves completion. Progress resets the short cadence.
use std::time::Duration;

pub(crate) struct Backoff {
    minimum: Duration,
    maximum: Duration,
    next: Duration,
    initial_polls: u8,
    remaining: u8,
}
impl Backoff {
    fn new(minimum: Duration, maximum: Duration) -> Option<Self> {
        if minimum.is_zero() || minimum > maximum || maximum > Duration::from_secs(1) {
            return None;
        }
        Some(Self {
            minimum,
            maximum,
            next: minimum,
            initial_polls: 0,
            remaining: 0,
        })
    }
    pub(crate) fn retirement() -> Self {
        Self::new(Duration::from_millis(5), Duration::from_millis(100)).unwrap()
    }
    #[cfg(feature = "commands")]
    pub(crate) fn command() -> Self {
        Self {
            initial_polls: 8,
            remaining: 8,
            ..Self::new(Duration::from_millis(1), Duration::from_millis(10)).unwrap()
        }
    }
    pub(crate) fn reset(&mut self) {
        self.next = self.minimum;
        self.remaining = self.initial_polls;
    }
    pub(crate) fn next(&mut self) -> Duration {
        if self.remaining > 0 {
            self.remaining -= 1;
            return self.minimum;
        }
        let delay = self.next;
        self.next = delay.saturating_mul(2).min(self.maximum);
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1529_backoff_bounds_growth_and_reset_for_every_admitted_millisecond_pair() {
        for minimum in [0, 1, 5, 10, 100, 1000, 1001] {
            for maximum in [0, 1, 5, 10, 100, 1000, 1001] {
                let backoff = Backoff::new(
                    Duration::from_millis(minimum),
                    Duration::from_millis(maximum),
                );
                assert_eq!(
                    backoff.is_some(),
                    minimum > 0 && minimum <= maximum && maximum <= 1000
                );
                if let Some(backoff) = backoff {
                    check_sequence(backoff, minimum, maximum);
                }
            }
        }
        assert!(Backoff::new(Duration::MAX, Duration::MAX).is_none());
    }
    #[cfg(feature = "commands")]
    #[test]
    fn t1529_short_commands_keep_one_millisecond_observation_before_backoff() {
        let mut backoff = Backoff::command();
        for _ in 0..9 {
            assert_eq!(backoff.next(), Duration::from_millis(1));
        }
        assert_eq!(backoff.next(), Duration::from_millis(2));
        backoff.reset();
        for _ in 0..9 {
            assert_eq!(backoff.next(), Duration::from_millis(1));
        }
    }
    fn check_sequence(mut backoff: Backoff, minimum: u64, maximum: u64) {
        let mut expected = minimum;
        for _ in 0..64 {
            assert_eq!(backoff.next(), Duration::from_millis(expected));
            expected = (expected * 2).min(maximum);
        }
        backoff.reset();
        assert_eq!(backoff.next(), Duration::from_millis(minimum));
    }
}
