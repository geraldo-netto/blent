//! Untrusted setup scheduling marker. It proves neither approval nor identity;
//! media authority still requires current local approval and mutual TLS proof.
use anyhow::{ensure, Result};

pub const BYTES: usize = 8;
pub fn encode(host: bool) -> [u8; BYTES] {
    [b'B', b'L', b'P', b'R', 1, u8::from(host), 0, 0]
}
pub fn decode(bytes: &[u8], expect_host: bool) -> Result<()> {
    ensure!(
        bytes == encode(expect_host),
        "invalid setup readiness marker"
    );
    Ok(())
}
