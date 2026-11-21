//! Explicit offline repair adapter. Parsing confirms intent before this workflow;
//! stable native store locks reject live owners and retain crash-recovery intent.
use crate::secure_connection::{
    identity_store::linux::LinuxIdentityStore, trust_store::LinuxTrustStore,
};
use anyhow::{ensure, Result};
use std::path::Path;

#[derive(Clone, Copy, clap::ValueEnum)]
pub(super) enum Reset {
    Trust,
    Identity,
}

pub(super) fn reset(path: &Path, selected: Reset) -> Result<()> {
    ensure!(path.is_absolute(), "state directory must be absolute");
    match selected {
        Reset::Trust => {
            LinuxTrustStore::open(path)?.reset()?;
            println!("Saved tablet approvals reset; host identity preserved. Reconnect and approve each tablet again.");
        }
        Reset::Identity => {
            LinuxIdentityStore::open(path)?.reset()?;
            println!("Host identity and saved tablet approvals reset. The next broker start will create a new identity.");
        }
    }
    Ok(())
}
