//! Bounded retry cadence; successful checks restore the daily interval.
use super::{Duration, INTERVAL};

const FIRST_RETRY: Duration = Duration::from_secs(60);
const MAX_RETRY: Duration = Duration::from_secs(60 * 60);

pub(super) struct Schedule {
    retry: Duration,
}

impl Schedule {
    pub(super) fn new() -> Self {
        Self { retry: FIRST_RETRY }
    }

    pub(super) fn delay(&mut self, success: bool) -> Duration {
        if success {
            self.retry = FIRST_RETRY;
            return INTERVAL;
        }
        let delay = self.retry;
        self.retry = (self.retry * 2).min(MAX_RETRY);
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1517_failure_backoff_is_bounded_and_success_resets_it() {
        for outcomes in 0..4096u16 {
            let mut schedule = Schedule::new();
            let mut failures = 0;
            for index in 0..12 {
                let success = outcomes & (1 << index) != 0;
                let expected = if success {
                    86_400
                } else {
                    (60u64 << failures.min(6)).min(3600)
                };
                assert_eq!(
                    schedule.delay(success),
                    Duration::from_secs(expected),
                    "T1517: outcomes={outcomes}, index={index}"
                );
                failures = if success { 0 } else { failures + 1 };
            }
        }
    }
}
