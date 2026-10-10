//! Construction adapters for shared Wi-Fi setup/reconnect policy.
use crate::usb::{Adb, Commands};
use anyhow::Result;
use blent_config::persistence_contract::{Snapshots, Transactions};
mod policy;
#[cfg(test)]
use policy::connected;
pub use policy::valid_address;

pub async fn reconnect<C: Commands, S: Snapshots + Transactions + Clone + 'static>(
    adb: &Adb<C>,
    store: &S,
) -> Option<String> {
    let owner = crate::config_jobs::Store::new(store.clone()).ok()?;
    policy::reconnect(adb, &owner).await
}
pub async fn setup<C: Commands, S: Snapshots + Transactions + Clone + 'static>(
    adb: &Adb<C>,
    store: &S,
    off: bool,
) -> Result<Option<String>> {
    let owner = crate::config_jobs::Store::new(store.clone())?;
    policy::setup(adb, &owner, off).await
}

#[cfg(test)]
mod tests;

pub struct Reconnect {
    path: std::path::PathBuf,
    adb: String,
}
impl Reconnect {
    pub fn new(path: std::path::PathBuf, adb: String) -> Self {
        Self { path, adb }
    }
    pub async fn connect(&self) -> Option<String> {
        reconnect(
            &crate::usb::Adb(crate::usb::NativeCommands(self.adb.clone().into())),
            &blent_config::storage::ConfigStore::new(self.path.clone()),
        )
        .await
    }
}

#[cfg(target_os = "linux")]
pub mod linux;
