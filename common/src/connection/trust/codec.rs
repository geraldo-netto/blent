//! Canonical disk record shared with Android. This is not a network message.
use super::{Snapshot, MAX_PEERS};
use crate::connection::pairing::PeerId;
use anyhow::{ensure, Result};
const MAGIC: &[u8; 8] = b"BLTR\0\0\0\x01";
const HEADER: usize = 88;
pub const MAX_RECORD: usize = HEADER + MAX_PEERS * 64;

pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>> {
    validate(snapshot)?;
    let mut bytes = Vec::with_capacity(HEADER + snapshot.peers.len() * 64);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(snapshot.local.as_str().as_bytes());
    bytes.extend_from_slice(&snapshot.revision.to_be_bytes());
    bytes.extend_from_slice(&(snapshot.peers.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&[0; 4]);
    for peer in &snapshot.peers {
        bytes.extend_from_slice(peer.as_str().as_bytes());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Snapshot> {
    ensure!(
        (HEADER..=MAX_RECORD).contains(&bytes.len()),
        "invalid trust record size"
    );
    ensure!(
        &bytes[..8] == MAGIC && bytes[84..88] == [0; 4],
        "invalid trust record header"
    );
    let count = u32::from_be_bytes(bytes[80..84].try_into()?) as usize;
    ensure!(
        count <= MAX_PEERS && bytes.len() == HEADER + count * 64,
        "invalid trust peer count"
    );
    let local = parse_peer(&bytes[8..72])?;
    let revision = u64::from_be_bytes(bytes[72..80].try_into()?);
    let peers = bytes[HEADER..]
        .chunks_exact(64)
        .map(parse_peer)
        .collect::<Result<Vec<_>>>()?;
    let snapshot = Snapshot {
        local,
        revision,
        peers,
    };
    validate(&snapshot)?;
    Ok(snapshot)
}
fn parse_peer(bytes: &[u8]) -> Result<PeerId> {
    PeerId::parse(std::str::from_utf8(bytes)?.to_owned())
}
fn validate(snapshot: &Snapshot) -> Result<()> {
    ensure!(
        (1..=i64::MAX as u64).contains(&snapshot.revision),
        "invalid trust revision"
    );
    ensure!(snapshot.peers.len() <= MAX_PEERS, "too many trusted peers");
    ensure!(
        snapshot
            .peers
            .windows(2)
            .all(|pair| pair[0].as_str() < pair[1].as_str()),
        "trust peers must be sorted and unique"
    );
    Ok(())
}
