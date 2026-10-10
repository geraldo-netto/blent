//! Bounded untrusted public certificate exchange preceding approval and TLS.
//! This only discovers a candidate identity; it grants no connection authority.
use anyhow::{ensure, Result};
pub const HEADER_BYTES: usize = 8;
pub const MAX_CERTIFICATE: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello {
    pub host: bool,
    pub certificate_bytes: u16,
}
impl Hello {
    pub fn encode(self) -> Result<[u8; HEADER_BYTES]> {
        ensure!(
            (1..=MAX_CERTIFICATE).contains(&(self.certificate_bytes as usize)),
            "invalid certificate extent"
        );
        let length = self.certificate_bytes.to_be_bytes();
        Ok([
            b'B',
            b'L',
            b'P',
            b'A',
            1,
            u8::from(self.host),
            length[0],
            length[1],
        ])
    }
    pub fn decode(bytes: &[u8], expect_host: bool) -> Result<Self> {
        ensure!(
            bytes.len() == HEADER_BYTES,
            "invalid identity header extent"
        );
        ensure!(
            &bytes[..5] == b"BLPA\x01" && bytes[5] == u8::from(expect_host),
            "unknown identity exchange"
        );
        let hello = Self {
            host: expect_host,
            certificate_bytes: u16::from_be_bytes([bytes[6], bytes[7]]),
        };
        hello.encode()?;
        Ok(hello)
    }
}
