//! Linux peer trust publication. A durable intent marker makes interrupted
//! updates recover the intended snapshot rather than restoring old approvals.
use super::private_directory::Directory;
use anyhow::{ensure, Result};
use blent_config::connection::trust::{
    codec::{self, MAX_RECORD},
    TrustStorage,
};
use std::io::Write;
use std::path::Path;

pub struct LinuxTrustStore {
    directory: Directory,
}
impl LinuxTrustStore {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            directory: Directory::open(path, c"trust.lock")?,
        })
    }
}
impl TrustStorage for LinuxTrustStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        self.recover()?;
        self.directory.read(c"trust.bin", MAX_RECORD)
    }
    fn replace(&mut self, record: &[u8]) -> Result<()> {
        ensure!(record.len() <= MAX_RECORD, "trust record exceeds limit");
        codec::decode(record)?;
        self.recover()?;
        let mut marker = self.directory.child(
            c"trust.pending",
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        marker.write_all(record)?;
        #[cfg(test)]
        tests::checkpoint("marker-sync")?;
        marker.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("intent-sync")?;
        self.directory.sync()?;
        self.write_record(record)?;
        self.retire_intent()
    }
}
impl LinuxTrustStore {
    fn write_record(&self, record: &[u8]) -> Result<()> {
        let mut file = self
            .directory
            .child(c"trust.next", libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)?;
        #[cfg(test)]
        tests::checkpoint("write")?;
        file.write_all(record)?;
        #[cfg(test)]
        tests::checkpoint("record-sync")?;
        file.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("rename")?;
        self.directory.rename(c"trust.next", c"trust.bin", true)?;
        #[cfg(test)]
        tests::checkpoint("publication-sync")?;
        self.directory.sync()
    }
}
mod recovery;
#[cfg(test)]
mod tests;
