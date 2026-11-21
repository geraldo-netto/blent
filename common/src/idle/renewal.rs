//! Publication scheduling for one original idle worker, independent of native IO.
//! This scalar history owns no grant, queue or resource. Call `published` only
//! after successful IO, using that attempt's start time so a blocked write does
//! not defer the next check. Rejection/revocation must bypass scheduling. It neither
//! extends a lease nor establishes native retirement. All u32 intervals and u64
//! clock values are compared without arithmetic overflow; clock reversal is due.

pub const RENEW_US: u64 = 250_000;

#[derive(Default)]
pub struct Renewal {
    previous: Option<(u32, u64)>,
}

impl Renewal {
    pub fn due(&self, interval_ms: u32, now_us: u64) -> bool {
        let Some((interval, published)) = self.previous else {
            return true;
        };
        interval != interval_ms
            || now_us
                .checked_sub(published)
                .is_none_or(|elapsed| elapsed >= RENEW_US)
    }

    pub fn published(&mut self, interval_ms: u32, now_us: u64) {
        self.previous = Some((interval_ms, now_us));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1673_renewal_deadline_changes_and_failed_publication() {
        let mut renewal = Renewal::default();
        assert!(renewal.due(500, 100));
        assert!(renewal.due(500, 100)); // No successful IO: no history advancement.
        renewal.published(500, 100);
        for now in [100, 101, 250_099] {
            assert!(!renewal.due(500, now));
        }
        for now in [0, 99, 250_100, 250_101, u64::MAX] {
            assert!(renewal.due(500, now));
        }
        for interval in [0, 200, 501, 1000, u32::MAX] {
            assert!(renewal.due(interval, 100));
        }
        renewal.published(1000, 200);
        assert!(!renewal.due(1000, 200));
        assert!(renewal.due(500, 200));
    }

    #[test]
    fn t1673_all_interval_boundaries_and_clock_exhaustion_remain_bounded() {
        for interval in [0, 1, 200, 500, 1000, u32::MAX] {
            let mut renewal = Renewal::default();
            renewal.published(interval, u64::MAX - 250_000);
            assert!(!renewal.due(interval, u64::MAX - 1));
            assert!(renewal.due(interval, u64::MAX));
            renewal.published(interval, u64::MAX);
            assert!(!renewal.due(interval, u64::MAX));
            assert!(renewal.due(interval, 0));
        }
    }
}
