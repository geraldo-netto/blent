//! Fixed, bounded envelopes shared with the Android connection adapter.
use anyhow::{ensure, Result};

/// Production v1 agrees this namespace for each fresh mutually authenticated
/// TLS stream. Local pairing epochs never cross the wire: peers retry separately.
/// Fresh TLS keys prevent cross-stream byte replay; callbacks additionally carry
/// the complete local pairing/resource lease. TLS resumption remains disabled.
pub const AUTHENTICATED_SESSION: u64 = 1;
pub const HEADER_BYTES: usize = 40;
pub const CHANNELS: u8 = 6;
pub const MAX_PAYLOAD: u32 = 65_536;
pub const MAX_CREDIT: u32 = 1_048_576;
pub const MAX_COUNTER: u64 = i64::MAX as u64;

/// Stable channel identifiers; these do not enable camera or microphone consent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Channel {
    Control = 1,
    Display = 2,
    Camera = 3,
    Microphone = 4,
    Speaker = 5,
    Management = 6,
}
impl TryFrom<u8> for Channel {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            1 => Self::Control,
            2 => Self::Display,
            3 => Self::Camera,
            4 => Self::Microphone,
            5 => Self::Speaker,
            6 => Self::Management,
            _ => anyhow::bail!("unknown connection channel"),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Data = 0,
    Credit = 1,
    Close = 2,
    Reset = 3,
}
impl TryFrom<u8> for Kind {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Data,
            1 => Self::Credit,
            2 => Self::Close,
            3 => Self::Reset,
            _ => anyhow::bail!("unknown connection frame kind"),
        })
    }
}
/// Session/stream identity is supplied by authenticated control, never inferred
/// from an incoming data frame. Sequence is per channel and direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub kind: Kind,
    pub channel: Channel,
    pub session: u64,
    pub generation: u64,
    pub sequence: u64,
    pub length: u32,
    pub credit: u32,
}
impl Header {
    pub fn validate(&self) -> Result<()> {
        ensure!((1..=MAX_COUNTER).contains(&self.session), "invalid session");
        ensure!(
            (1..=MAX_COUNTER).contains(&self.generation),
            "invalid channel generation"
        );
        ensure!(self.sequence <= MAX_COUNTER, "invalid sequence");
        self.validate_body()
    }
    fn validate_body(&self) -> Result<()> {
        match self.kind {
            Kind::Data => {
                ensure!(
                    (1..=MAX_PAYLOAD).contains(&self.length),
                    "invalid payload length"
                );
                ensure!(self.credit == 0, "data frame grants credit");
            }
            Kind::Credit => {
                ensure!(self.length == 0, "credit frame has payload");
                ensure!((1..=MAX_CREDIT).contains(&self.credit), "invalid credit");
            }
            Kind::Close | Kind::Reset => {
                ensure!(
                    self.length == 0 && self.credit == 0,
                    "terminal frame has body"
                );
            }
        }
        Ok(())
    }
    /// Validates before changing caller-owned output; no payload allocation.
    pub fn encode(&self, output: &mut [u8]) -> Result<()> {
        ensure!(output.len() == HEADER_BYTES, "invalid header buffer length");
        self.validate()?;
        output[..8].copy_from_slice(&[
            b'B',
            b'L',
            b'N',
            b'T',
            1,
            self.kind as u8,
            self.channel as u8,
            0,
        ]);
        output[8..16].copy_from_slice(&self.session.to_be_bytes());
        output[16..24].copy_from_slice(&self.generation.to_be_bytes());
        output[24..32].copy_from_slice(&self.sequence.to_be_bytes());
        output[32..36].copy_from_slice(&self.length.to_be_bytes());
        output[36..40].copy_from_slice(&self.credit.to_be_bytes());
        Ok(())
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        ensure!(input.len() == HEADER_BYTES, "invalid header length");
        ensure!(
            &input[..5] == b"BLNT\x01" && input[7] == 0,
            "invalid header prefix"
        );
        let header = Self {
            kind: Kind::try_from(input[5])?,
            channel: Channel::try_from(input[6])?,
            session: u64::from_be_bytes(input[8..16].try_into()?),
            generation: u64::from_be_bytes(input[16..24].try_into()?),
            sequence: u64::from_be_bytes(input[24..32].try_into()?),
            length: u32::from_be_bytes(input[32..36].try_into()?),
            credit: u32::from_be_bytes(input[36..40].try_into()?),
        };
        header.validate()?;
        Ok(header)
    }
}
