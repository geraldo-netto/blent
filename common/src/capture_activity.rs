//! Capture activity records, independent of native capture and process storage.
//! A peer echoes the exact record only after that transition has completed.
use anyhow::{ensure, Context, Result};

pub const RECORD_BYTES: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    sequence: u64,
    active: bool,
}

impl Request {
    pub fn after(previous: u64, active: bool) -> Result<Self> {
        Ok(Self {
            sequence: previous
                .checked_add(1)
                .context("capture activity exhausted")?,
            active,
        })
    }

    pub fn sequence(self) -> u64 {
        self.sequence
    }

    pub fn active(self) -> bool {
        self.active
    }

    pub fn encode(self) -> [u8; RECORD_BYTES] {
        let mut bytes = [0; RECORD_BYTES];
        bytes[..8].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[8] = u8::from(self.active);
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() == RECORD_BYTES,
            "invalid capture activity length"
        );
        let sequence = u64::from_be_bytes(bytes[..8].try_into()?);
        ensure!(sequence != 0, "invalid capture activity sequence");
        ensure!(bytes[8] <= 1, "invalid capture activity flag");
        Ok(Self {
            sequence,
            active: bytes[8] != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1023_fixed_vectors_and_exhaustion() {
        let first = Request::after(0, true).unwrap();
        assert_eq!(first.encode(), [0, 0, 0, 0, 0, 0, 0, 1, 1]);
        assert!(first.active());
        assert_eq!(first.sequence(), 1);
        let last = Request::after(u64::MAX - 1, false).unwrap();
        assert_eq!(last.encode(), [255, 255, 255, 255, 255, 255, 255, 255, 0]);
        assert_eq!(Request::decode(&last.encode()).unwrap(), last);
        assert!(Request::after(u64::MAX, true).is_err());
    }

    #[test]
    fn t1023_bounded_record_mutations() {
        for length in 0..=32 {
            assert_eq!(Request::decode(&vec![1; length]).is_ok(), length == 9);
        }
        for flag in 0..=255 {
            let mut bytes = [0; RECORD_BYTES];
            bytes[7] = 1;
            bytes[8] = flag;
            assert_eq!(Request::decode(&bytes).is_ok(), flag <= 1);
            bytes[7] = 0;
            assert!(Request::decode(&bytes).is_err());
        }
        for sequence in [0, 1, 127, 255, 256, u32::MAX as u64, u64::MAX - 1] {
            for active in [true, false] {
                let request = Request::after(sequence, active).unwrap();
                assert_eq!(Request::decode(&request.encode()).unwrap(), request);
            }
        }
    }
}
