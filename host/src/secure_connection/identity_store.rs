//! T867 durable identity policy. Storage owns private, exclusive, crash-safe I/O;
//! stock TLS owns certificate/key validation. Neither contract exposes OS types.
use super::{Identity, IdentityRecord, MAX_CERTIFICATE, MAX_PRIVATE_KEY};
use anyhow::{ensure, Result};

const MAGIC: &[u8; 8] = b"BLID\0\0\0\x01";
pub const MAX_RECORD: usize = 16 + MAX_CERTIFICATE + MAX_PRIVATE_KEY;

/// One exclusive owner for the entire load/create operation. An absent identity
/// is conclusive only when no interrupted initialization exists. reserve() must
/// durably record initialization BEFORE key generation. publish() atomically
/// installs the complete record without overwriting any existing identity.
/// Any failure preserves existing/pending state; callers must not retry with a
/// replacement key or use a generated identity before publication succeeds.
pub trait IdentityStorage {
    fn load(&mut self) -> Result<Option<Vec<u8>>>;
    fn reserve(&mut self) -> Result<()>;
    fn publish(&mut self, record: &[u8]) -> Result<()>;
}

pub fn load_or_create(
    storage: &mut dyn IdentityStorage,
    generate: impl FnOnce() -> Result<IdentityRecord>,
) -> Result<Identity> {
    if let Some(bytes) = storage.load()? {
        return Identity::load(decode(&bytes)?);
    }
    storage.reserve()?;
    let record = generate()?;
    let bytes = encode(&record)?;
    let identity = Identity::load(record)?;
    storage.publish(&bytes)?;
    Ok(identity)
}

fn lengths(certificate: usize, key: usize) -> Result<()> {
    ensure!(
        (1..=MAX_CERTIFICATE).contains(&certificate),
        "invalid identity certificate size"
    );
    ensure!(
        (1..=MAX_PRIVATE_KEY).contains(&key),
        "invalid identity key size"
    );
    Ok(())
}
fn encode(record: &IdentityRecord) -> Result<Vec<u8>> {
    lengths(record.certificate.len(), record.private_key.len())?;
    let mut bytes = Vec::with_capacity(16 + record.certificate.len() + record.private_key.len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(record.certificate.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&(record.private_key.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&record.certificate);
    bytes.extend_from_slice(&record.private_key);
    Ok(bytes)
}
fn decode(bytes: &[u8]) -> Result<IdentityRecord> {
    ensure!(
        (16..=MAX_RECORD).contains(&bytes.len()),
        "invalid identity record size"
    );
    ensure!(&bytes[..8] == MAGIC, "invalid identity record version");
    let certificate = u32::from_be_bytes(bytes[8..12].try_into()?) as usize;
    let key = u32::from_be_bytes(bytes[12..16].try_into()?) as usize;
    lengths(certificate, key)?;
    ensure!(
        bytes.len() == 16 + certificate + key,
        "invalid identity record length"
    );
    Ok(IdentityRecord {
        certificate: bytes[16..16 + certificate].to_vec(),
        private_key: bytes[16 + certificate..].to_vec(),
    })
}

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(test)]
mod tests;
