//! Linux peer trust publication. A durable intent marker makes interrupted
//! updates fail closed rather than restoring an older approval after revocation.
use super::private_directory::Directory;
use anyhow::{ensure, Result};
use blent_config::connection::trust::{codec::MAX_RECORD, TrustStorage};
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
    fn settled(&self) -> Result<()> {
        self.directory.ensure_absent(c"trust.pending")?;
        self.directory.ensure_absent(c"trust.next")
    }
}
impl TrustStorage for LinuxTrustStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        self.settled()?;
        self.directory.read(c"trust.bin", MAX_RECORD)
    }
    fn replace(&mut self, record: &[u8]) -> Result<()> {
        ensure!(record.len() <= MAX_RECORD, "trust record exceeds limit");
        self.settled()?;
        let marker = self.directory.child(
            c"trust.pending",
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        #[cfg(test)]
        tests::checkpoint("marker-sync")?;
        marker.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("intent-sync")?;
        self.directory.sync()?;
        self.write_record(record)?;
        #[cfg(test)]
        tests::checkpoint("remove-marker")?;
        self.directory.remove(c"trust.pending")?;
        #[cfg(test)]
        tests::checkpoint("retirement-sync")?;
        self.directory.sync()
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
#[cfg(test)]
mod tests;
