//! Byte budgets are independent of process and pipe implementations.
use std::io;
const MIB: usize = 1024 * 1024;
pub(super) struct Limits {
    pub streams: [usize; 2],
    pub total: usize,
}
impl Limits {
    pub fn environment() -> io::Result<Self> {
        use crate::retirement_limits::configured;
        Ok(Self {
            streams: [
                configured("BLENT_COMMAND_STDOUT_BYTES", 4 * MIB, 64 * MIB)?,
                configured("BLENT_COMMAND_STDERR_BYTES", MIB, 64 * MIB)?,
            ],
            total: configured("BLENT_COMMAND_TOTAL_BYTES", 5 * MIB, 128 * MIB)?,
        })
    }
}
pub(super) struct Bytes {
    pub streams: [Vec<u8>; 2],
    limits: Limits,
    used: usize,
}
impl Bytes {
    pub fn new(limits: Limits) -> Self {
        Self {
            streams: Default::default(),
            limits,
            used: 0,
        }
    }
    pub fn append(&mut self, index: usize, chunk: &[u8]) -> io::Result<()> {
        let limit = self.limits.streams[index].min(self.limits.total);
        let buffer = &mut self.streams[index];
        if chunk.len() > limit - buffer.len() || chunk.len() > self.limits.total - self.used {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "command output exceeds byte budget",
            ));
        }
        let required = buffer.len() + chunk.len();
        if buffer.capacity() < required {
            let capacity = buffer.capacity().saturating_mul(2).max(required).min(limit);
            buffer
                .try_reserve_exact(capacity - buffer.len())
                .map_err(io::Error::other)?;
        }
        buffer.extend_from_slice(chunk);
        self.used += chunk.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t815_stream_and_total_boundaries_preserve_only_admitted_bytes() {
        for limit in [1, 2, 7, 32, 4096, 65536] {
            for split in [0, 1, limit / 2, limit] {
                let mut bytes = Bytes::new(Limits {
                    streams: [limit, limit],
                    total: limit,
                });
                bytes.append(0, &vec![1; split]).unwrap();
                bytes.append(1, &vec![2; limit - split]).unwrap();
                assert!(bytes.append(0, &[3]).is_err());
                assert!(bytes.append(1, &[3]).is_err());
                assert_eq!(bytes.streams, [vec![1; split], vec![2; limit - split]]);
                assert!(bytes
                    .streams
                    .iter()
                    .all(|stream| stream.capacity() <= limit));
            }
            let mut bytes = Bytes::new(Limits {
                streams: [limit, limit],
                total: 2 * limit,
            });
            for _ in 0..limit {
                bytes.append(0, &[7]).unwrap();
            }
            assert!(bytes.append(0, &[8]).is_err());
            bytes.append(1, &vec![9; limit]).unwrap();
        }
    }
}
