//! Retained cleanup owns a fixed-size cadence; elapsed time never settles work.
use std::time::Duration;

pub(crate) struct Retry {
    delay: Duration,
}

impl Retry {
    pub(crate) fn new() -> Self {
        Self {
            delay: Duration::from_millis(100),
        }
    }

    pub(crate) fn next(&mut self) -> Duration {
        let delay = self.delay;
        self.delay = (self.delay * 2).min(Duration::from_secs(5));
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1655_retry_reaches_cap_and_new_owners_start_independently() {
        let mut retry = Retry::new();
        for expected in [100, 200, 400, 800, 1600, 3200] {
            assert_eq!(retry.next(), Duration::from_millis(expected));
        }
        for _ in 0..4096 {
            assert_eq!(retry.next(), Duration::from_secs(5));
            assert_eq!(Retry::new().next(), Duration::from_millis(100));
        }
    }
}
