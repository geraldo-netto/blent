//! Linux private persistence adapter. The broker supplies an existing private
//! directory; all child operations use its pinned descriptor and fixed names.
//! No path selection, user-directory creation or Windows support is implied.
use super::{IdentityStorage, MAX_RECORD};
use crate::secure_connection::private_directory::Directory;
use anyhow::{ensure, Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct LinuxIdentityStore {
    directory: Directory,
    pending: Option<File>,
}
impl LinuxIdentityStore {
    /// Keep this owner alive while consuming its identity. Fail immediately if
    /// another process owns the store. Never unlink/replace the stable lock.
    pub fn open(path: &Path) -> Result<Self> {
        let directory = Directory::open(path, c"identity.lock")?;
        Ok(Self {
            directory,
            pending: None,
        })
    }
    fn read_identity(&self) -> Result<Option<Vec<u8>>> {
        self.directory.read(c"identity.bin", MAX_RECORD)
    }
    fn no_pending(&self) -> Result<()> {
        self.directory.ensure_absent(c"identity.pending")
    }
}
impl IdentityStorage for LinuxIdentityStore {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        // A pending file is never ignored, even if someone supplied a final file.
        self.no_pending()?;
        self.read_identity()
    }
    fn reserve(&mut self) -> Result<()> {
        ensure!(self.read_identity()?.is_none(), "identity already exists");
        self.no_pending()?;
        let file = self.directory.child(
            c"identity.pending",
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
        )?;
        #[cfg(test)]
        tests::checkpoint("pending-sync")?;
        file.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("reserve-sync")?;
        self.directory.sync()?;
        #[cfg(test)]
        tests::checkpoint("reserved")?;
        self.pending = Some(file);
        Ok(())
    }
    fn publish(&mut self, record: &[u8]) -> Result<()> {
        ensure!(record.len() <= MAX_RECORD, "identity record exceeds limit");
        let mut file = self
            .pending
            .take()
            .context("identity initialization not reserved")?;
        #[cfg(test)]
        tests::checkpoint("write")?;
        file.write_all(record)?;
        #[cfg(test)]
        tests::checkpoint("record-sync")?;
        file.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("rename")?;
        // NOREPLACE preserves unexpected existing state, including dangling links.
        self.directory
            .rename(c"identity.pending", c"identity.bin", false)?;
        #[cfg(test)]
        tests::checkpoint("publish-sync")?;
        self.directory.sync()?;
        #[cfg(test)]
        tests::checkpoint("published")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
