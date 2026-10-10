//! Immutable selection contracts. Policy and worker implementations depend on
//! these values; the values never depend on a media/session implementation.
use blent_config::negotiation::DecoderCapabilities;

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub format: (u32, u32, u32, u32, u32),
    pub epoch: u64,
    pub decoders: Option<DecoderCapabilities>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selected {
    pub workers: u32,
    pub key: Key,
    pub encoder: String,
    pub reason: String,
    pub verified: bool,
    /// Planned tuning/verification; hide intermediate desktop frames on capable clients.
    pub calibrating: bool,
    pub decoder: Option<blent_config::negotiation::DecoderChoice>,
}
