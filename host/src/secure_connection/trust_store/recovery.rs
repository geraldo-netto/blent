//! Recovery borrows the store's pinned directory and original exclusive lock.
//! Invalid intent never becomes old approval. Explicit reset keeps identity.
use super::*;

impl LinuxTrustStore {
    pub(super) fn recover(&self) -> Result<()> {
        self.directory.ensure_absent(c"identity.reset")?;
        if self.directory.read(c"trust.reset", 0)?.is_some() {
            self.directory.sync()?;
            return self.finish_reset();
        }
        if let Some(intended) = self.directory.read(c"trust.pending", MAX_RECORD)? {
            codec::decode(&intended)?;
            self.directory.sync()?;
            self.directory.remove_if_present(c"trust.next")?;
            self.write_record(&intended)?;
            self.retire_intent()?;
        }
        self.directory.ensure_absent(c"trust.next")
    }

    pub(super) fn retire_intent(&self) -> Result<()> {
        #[cfg(test)]
        tests::checkpoint("remove-marker")?;
        self.directory.remove(c"trust.pending")?;
        #[cfg(test)]
        tests::checkpoint("retirement-sync")?;
        self.directory.sync()
    }

    /// Destructive offline authority: only an explicitly confirmed repair entry
    /// may call this. The original lock excludes live connection/trust owners.
    pub fn reset(&mut self) -> Result<()> {
        if self.directory.read(c"trust.reset", 0)?.is_none() {
            let marker = self.directory.child(
                c"trust.reset",
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )?;
            marker.sync_all()?;
        }
        #[cfg(test)]
        tests::checkpoint("reset-intent-sync")?;
        self.directory.sync()?;
        self.finish_reset()
    }

    fn finish_reset(&self) -> Result<()> {
        for name in [c"trust.bin", c"trust.next", c"trust.pending"] {
            self.directory.remove_if_present(name)?;
            #[cfg(test)]
            tests::checkpoint(name.to_str().unwrap())?;
        }
        self.directory.sync()?;
        #[cfg(test)]
        tests::checkpoint("reset-remove")?;
        self.directory.remove(c"trust.reset")?;
        #[cfg(test)]
        tests::checkpoint("reset-retirement-sync")?;
        self.directory.sync()
    }
}
