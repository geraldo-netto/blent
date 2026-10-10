use super::*;
/// Fixed storage; one returned record per call. The caller owns any coalesced
/// remainder. Malformed input poisons the stream; no prefix resynchronization.
pub struct Decoder {
    bytes: [u8; MAX_RECORD],
    used: usize,
    target: usize,
    failed: bool,
    ended: bool,
}
impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Decoder {
    pub fn new() -> Self {
        Self {
            bytes: [0; MAX_RECORD],
            used: 0,
            target: HEADER_BYTES,
            failed: false,
            ended: false,
        }
    }
    pub fn feed(&mut self, input: &[u8]) -> Result<(usize, Option<Message>)> {
        ensure!(!self.failed && !self.ended, "management decoder retired");
        let result = self.consume(input);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn consume(&mut self, input: &[u8]) -> Result<(usize, Option<Message>)> {
        let mut consumed = 0;
        loop {
            let count = (self.target - self.used).min(input.len() - consumed);
            self.bytes[self.used..self.used + count]
                .copy_from_slice(&input[consumed..consumed + count]);
            self.used += count;
            consumed += count;
            if self.used < self.target {
                return Ok((consumed, None));
            }
            if self.target == HEADER_BYTES {
                self.target =
                    HEADER_BYTES + codec::Header::decode(&self.bytes[..HEADER_BYTES])?.body;
            }
            if self.used == self.target {
                let message = Message::decode(&self.bytes[..self.used])?;
                self.used = 0;
                self.target = HEADER_BYTES;
                return Ok((consumed, Some(message)));
            }
        }
    }
    pub fn end(&mut self) -> Result<()> {
        ensure!(!self.failed && !self.ended, "management decoder retired");
        self.ended = true;
        ensure!(self.used == 0, "truncated management record");
        Ok(())
    }
}
