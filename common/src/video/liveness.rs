//! Display transport liveness, independent of frames, codec and native resources.
use anyhow::{ensure, Result};
use std::time::Duration;

/// Four-byte BE length 2, this type byte, then the exact version byte 1.
pub const PACKET_TYPE: u8 = 2;
pub const PAYLOAD: [u8; 1] = [1];
pub const DEFAULT_INTERVAL_MS: usize = 2000;
pub const MAX_INTERVAL_MS: usize = 5000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interval(Duration);
impl Interval {
    pub fn duration(self) -> Duration {
        self.0
    }
}

/// Bound traffic/wake frequency while staying below Android's 10-second read deadline.
pub fn interval(milliseconds: usize) -> Result<Interval> {
    ensure!(
        (100..=MAX_INTERVAL_MS).contains(&milliseconds),
        "invalid video liveness interval"
    );
    Ok(Interval(Duration::from_millis(milliseconds as u64)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1038_liveness_interval_bounds_and_invalid_values() {
        for value in 0..=10000 {
            assert_eq!(interval(value).is_ok(), (100..=5000).contains(&value));
        }
        assert!(interval(usize::MAX).is_err());
        assert_eq!(
            interval(DEFAULT_INTERVAL_MS).unwrap().duration(),
            Duration::from_secs(2)
        );
    }
}
