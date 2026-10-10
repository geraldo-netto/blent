//! Bounded v1/v2 PCM framing on a session-authenticated connection, not encryption.
use super::{AudioProfile, Direction, BLOCK_FRAMES, MAX_SAMPLES, SAMPLE_RATE};
use crate::credentials::{random_token_using, token_matches, Entropy};
use anyhow::{ensure, Result};

pub const HELLO_BYTES: usize = 92;
pub const FRAME_HEADER_BYTES: usize = 28;
const MAGIC: &[u8; 8] = b"BLAUD001";
const CLOCK_MAGIC: &[u8; 8] = b"BLAUD002";

/// Secret-bearing grant. Never log, serialize into configuration or reuse it.
#[derive(Clone)]
pub struct AudioGrant {
    pub(super) profile: AudioProfile,
    pub(super) generation: u64,
    token: String,
    pub(super) clocked: bool,
}

impl AudioGrant {
    pub(super) fn new(profile: AudioProfile, generation: u64, entropy: Entropy) -> Result<Self> {
        Ok(Self {
            profile,
            generation,
            token: random_token_using(entropy)?,
            clocked: false,
        })
    }

    pub fn profile(&self) -> AudioProfile {
        self.profile
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Deliver only through the already-authenticated control connection.
    pub fn hello(&self) -> [u8; HELLO_BYTES] {
        let mut bytes = [0; HELLO_BYTES];
        bytes[..8].copy_from_slice(if self.clocked { CLOCK_MAGIC } else { MAGIC });
        bytes[8..72].copy_from_slice(self.token.as_bytes());
        bytes[72..80].copy_from_slice(&self.generation.to_be_bytes());
        bytes[80] = self.profile.direction as u8;
        bytes[81] = self.profile.direction.channels() as u8;
        bytes[82..86].copy_from_slice(&SAMPLE_RATE.to_be_bytes());
        bytes[86..88].copy_from_slice(&(BLOCK_FRAMES as u16).to_be_bytes());
        bytes[88] = self.profile.processing as u8;
        bytes[89] = u8::from(self.profile.background);
        bytes[90..92].copy_from_slice(&self.profile.buffer_ms.to_be_bytes());
        bytes
    }

    /// Fixed-length negotiation before reading/allocating any PCM payload.
    pub fn authenticate(&self, bytes: &[u8]) -> Result<FrameReader> {
        ensure!(bytes.len() == HELLO_BYTES, "invalid audio handshake length");
        let token = std::str::from_utf8(&bytes[8..72])?;
        ensure!(
            token_matches(&self.token, token),
            "audio authentication failed"
        );
        let expected = self.hello();
        ensure!(
            bytes[..8] == expected[..8] && bytes[72..] == expected[72..],
            "audio negotiation mismatch"
        );
        Ok(FrameReader {
            generation: self.generation,
            direction: self.profile.direction,
            previous: None,
            clocked: self.clocked,
        })
    }

    /// Sequence starts at zero and cannot wrap. Timestamp is sender monotonic µs.
    pub fn encode(&self, sequence: u64, timestamp_us: u64, samples: &[i16]) -> Result<Vec<u8>> {
        self.encode_with_clock(sequence, timestamp_us, samples, None)
    }

    pub fn encode_with_clock(
        &self,
        sequence: u64,
        timestamp_us: u64,
        samples: &[i16],
        clock: Option<super::ClockSample>,
    ) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.encode_into(sequence, timestamp_us, samples, clock, &mut bytes)?;
        Ok(bytes)
    }

    pub fn encode_into(
        &self,
        sequence: u64,
        timestamp_us: u64,
        samples: &[i16],
        clock: Option<super::ClockSample>,
        bytes: &mut Vec<u8>,
    ) -> Result<()> {
        ensure!(sequence < u64::MAX, "audio sequence exhausted");
        ensure!(
            samples.len() == BLOCK_FRAMES * self.profile.direction.channels(),
            "invalid PCM block size"
        );
        let clock_bytes = if self.clocked { super::CLOCK_BYTES } else { 0 };
        let size = samples.len() * 2 + clock_bytes;
        let encoded_clock = if self.clocked {
            Some(super::ClockSample::encode(clock)?)
        } else {
            None
        };
        bytes.resize(FRAME_HEADER_BYTES + size, 0);
        bytes[27] = 0;
        if let Some(clock) = encoded_clock {
            bytes[28..52].copy_from_slice(&clock);
        }
        bytes[..8].copy_from_slice(&self.generation.to_be_bytes());
        bytes[8..16].copy_from_slice(&sequence.to_be_bytes());
        bytes[16..24].copy_from_slice(&timestamp_us.to_be_bytes());
        bytes[24..26].copy_from_slice(&(size as u16).to_be_bytes());
        bytes[26] = self.profile.direction as u8;
        for (sample, output) in samples
            .iter()
            .zip(bytes[28 + clock_bytes..].as_chunks_mut::<2>().0.iter_mut())
        {
            output.copy_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PcmBlock {
    pub(super) samples: [i16; MAX_SAMPLES],
    pub(super) channels: usize,
    pub discontinuity: bool,
    pub clock: Option<super::ClockSample>,
}

impl PcmBlock {
    /// Trusted native PCM still receives exact size/channel validation.
    pub fn from_le_bytes(direction: Direction, bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() == BLOCK_FRAMES * direction.channels() * 2,
            "invalid PCM block length"
        );
        let mut block = Self {
            samples: [0; MAX_SAMPLES],
            channels: direction.channels(),
            discontinuity: false,
            clock: None,
        };
        for (sample, bytes) in block
            .samples
            .iter_mut()
            .zip(bytes.as_chunks::<2>().0.iter())
        {
            *sample = i16::from_le_bytes([bytes[0], bytes[1]]);
        }
        Ok(block)
    }

    pub fn samples(&self) -> &[i16] {
        &self.samples[..BLOCK_FRAMES * self.channels]
    }
}

/// Borrowed PCM validated together with its clock and sequence. No native types.
pub struct PcmView<'a> {
    pub pcm: &'a [u8],
    pub clock: Option<super::ClockSample>,
    pub discontinuity: bool,
}

pub struct FrameReader {
    generation: u64,
    direction: Direction,
    previous: Option<(u64, u64)>,
    clocked: bool,
}

/// Sender sequence ownership, including capture discontinuities and exhaustion.
pub struct FrameWriter {
    grant: AudioGrant,
    next: u64,
    previous_time: Option<u64>,
}
impl FrameWriter {
    pub fn new(grant: AudioGrant) -> Self {
        Self {
            grant,
            next: 0,
            previous_time: None,
        }
    }
    pub fn encode(&mut self, block: &PcmBlock, timestamp_us: u64) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.encode_into(block, timestamp_us, &mut bytes)?;
        Ok(bytes)
    }
    pub fn encode_into(
        &mut self,
        block: &PcmBlock,
        timestamp_us: u64,
        bytes: &mut Vec<u8>,
    ) -> Result<()> {
        ensure!(
            self.previous_time.is_none_or(|last| timestamp_us > last),
            "audio clock did not advance"
        );
        let sequence = if block.discontinuity && self.next != 0 {
            self.next
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("audio sequence exhausted"))?
        } else {
            self.next
        };
        self.grant
            .encode_into(sequence, timestamp_us, block.samples(), block.clock, bytes)?;
        self.next = sequence + 1; // encode rejects u64::MAX before state changes.
        self.previous_time = Some(timestamp_us);
        Ok(())
    }
}

impl FrameReader {
    /// Check the entire fixed header before allocating or waiting for payload.
    /// Adapter must use an exact, bounded read and retire the connection on error.
    pub fn payload_bytes(&self, header: &[u8]) -> Result<usize> {
        ensure!(
            header.len() == FRAME_HEADER_BYTES,
            "invalid audio frame header"
        );
        ensure!(
            header[..8] == self.generation.to_be_bytes(),
            "stale audio generation"
        );
        ensure!(
            header[26] == self.direction as u8 && header[27] == 0,
            "invalid audio frame flags/direction"
        );
        let size = u16::from_be_bytes(header[24..26].try_into()?) as usize;
        ensure!(
            size == BLOCK_FRAMES * self.direction.channels() * 2
                + if self.clocked { super::CLOCK_BYTES } else { 0 },
            "invalid audio payload size"
        );
        self.sequence(header)?;
        Ok(size)
    }

    fn sequence(&self, header: &[u8]) -> Result<(u64, u64, bool)> {
        let sequence = u64::from_be_bytes(header[8..16].try_into()?);
        let timestamp = u64::from_be_bytes(header[16..24].try_into()?);
        ensure!(sequence < u64::MAX, "audio sequence exhausted");
        let discontinuity = if let Some((last, time)) = self.previous {
            ensure!(
                sequence > last && timestamp > time,
                "replayed or reordered audio"
            );
            sequence != last + 1
        } else {
            ensure!(sequence == 0, "audio must start at sequence zero");
            false
        };
        Ok((sequence, timestamp, discontinuity))
    }

    /// Validate borrowed storage without materializing native PCM. State advances
    /// only after all header, size and clock checks succeed. Adapter retires a
    /// connection on partial I/O; the view cannot outlive its input storage.
    pub fn validate_parts<'a>(&mut self, header: &[u8], payload: &'a [u8]) -> Result<PcmView<'a>> {
        let size = self.payload_bytes(header)?;
        ensure!(
            payload.len() == size,
            "truncated or oversized audio payload"
        );
        let (sequence, timestamp, discontinuity) = self.sequence(header)?;
        let clock_bytes = if self.clocked { super::CLOCK_BYTES } else { 0 };
        let clock = if self.clocked {
            super::ClockSample::decode(&payload[..clock_bytes])?
        } else {
            None
        };
        self.previous = Some((sequence, timestamp));
        Ok(PcmView {
            pcm: &payload[clock_bytes..],
            clock,
            discontinuity,
        })
    }

    /// No state advances until header and complete payload both validate.
    pub fn decode(&mut self, bytes: &[u8]) -> Result<PcmBlock> {
        ensure!(bytes.len() >= FRAME_HEADER_BYTES, "truncated audio frame");
        let view =
            self.validate_parts(&bytes[..FRAME_HEADER_BYTES], &bytes[FRAME_HEADER_BYTES..])?;
        let mut block = PcmBlock {
            samples: [0; MAX_SAMPLES],
            channels: self.direction.channels(),
            discontinuity: view.discontinuity,
            clock: view.clock,
        };
        for (output, input) in block
            .samples
            .iter_mut()
            .zip(view.pcm.as_chunks::<2>().0.iter())
        {
            *output = i16::from_le_bytes([input[0], input[1]]);
        }
        Ok(block)
    }
}

#[cfg(test)]
mod writer_tests {
    use super::*;
    #[test]
    fn t719_sender_preserves_first_sequence_gaps_and_atomic_rejection() {
        let grant = AudioGrant::new(
            AudioProfile::new(Direction::Speakers),
            1,
            crate::credentials::tests::entropy,
        )
        .unwrap();
        let mut reader = grant.authenticate(&grant.hello()).unwrap();
        let mut writer = FrameWriter::new(grant.clone());
        let mut block = PcmBlock::from_le_bytes(Direction::Speakers, &[0; 1920]).unwrap();
        block.discontinuity = true;
        assert!(
            !reader
                .decode(&writer.encode(&block, 10).unwrap())
                .unwrap()
                .discontinuity
        );
        for time in [0, 9, 10] {
            assert!(writer.encode(&block, time).is_err());
        }
        assert!(
            reader
                .decode(&writer.encode(&block, 11).unwrap())
                .unwrap()
                .discontinuity
        );
        block.discontinuity = false;
        assert!(
            !reader
                .decode(&writer.encode(&block, 12).unwrap())
                .unwrap()
                .discontinuity
        );
        let mono = PcmBlock::from_le_bytes(Direction::Microphone, &[0; 960]).unwrap();
        assert!(writer.encode(&mono, 13).is_err());
        writer.next = u64::MAX - 1;
        assert!(writer.encode(&block, 13).is_ok());
        assert!(writer.encode(&block, 14).is_err());
        block.discontinuity = true;
        assert!(writer.encode(&block, 14).is_err());
        let mut final_clock = FrameWriter::new(grant);
        assert!(final_clock.encode(&block, u64::MAX).is_ok());
        assert!(final_clock.encode(&block, u64::MAX).is_err());
    }
}
