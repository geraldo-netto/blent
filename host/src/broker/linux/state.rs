//! Fixed-name private state adapters; run only inside the authority worker.
use crate::{
    broker::authority_worker::Initialized,
    secure_connection::{
        identity_store::{linux::LinuxIdentityStore, load_or_create},
        trust_store::LinuxTrustStore,
        IdentityRecord, PeerCertificate,
    },
};
use anyhow::Result;
use blent_config::connection::trust::Trust;
use std::{path::Path, sync::Arc};

/// Caller supplies an existing private directory. Store adapters reject unsafe
/// paths and interrupted transactions and hold stable exclusive lifetime locks.
pub fn open(path: &Path, capacity: usize) -> Result<Initialized<LinuxTrustStore>> {
    let mut identity_owner = LinuxIdentityStore::open(path)?;
    let identity = load_or_create(&mut identity_owner, IdentityRecord::generate)?;
    let certificate = PeerCertificate::parse(identity.certificate().to_vec())?;
    let peer = certificate.peer_id();
    let trust = Trust::open(LinuxTrustStore::open(path)?, peer, capacity)?;
    Ok(Initialized {
        identity: Arc::new(identity),
        trust,
        identity_owner: Arc::new(identity_owner),
    })
}

#[cfg(test)]
mod tests;
