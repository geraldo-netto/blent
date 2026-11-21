//! A complete unpublished identity can finish without key rotation. Explicit
//! identity reset also retires all approvals bound to the former certificate.
use super::*;
use crate::secure_connection::{identity_store::decode, Identity};

impl LinuxIdentityStore {
    pub(super) fn recover(&self) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "identity initialization still reserved"
        );
        if self.directory.read(c"identity.reset", 0)?.is_some() {
            return self.reset_records();
        }
        let Some(record) = self.directory.read(c"identity.pending", MAX_RECORD)? else {
            return Ok(());
        };
        ensure!(
            self.read_identity()?.is_none(),
            "conflicting identity records; explicit reset required"
        );
        Identity::load(decode(&record)?)?;
        self.directory
            .child(c"identity.pending", libc::O_RDONLY)?
            .sync_all()?;
        #[cfg(test)]
        tests::checkpoint("recovery-rename")?;
        self.directory
            .rename(c"identity.pending", c"identity.bin", false)?;
        #[cfg(test)]
        tests::checkpoint("recovery-sync")?;
        self.directory.sync()
    }

    /// Only an explicitly confirmed offline repair may destroy identity. Both
    /// stable locks stay held until all identity-bound approvals have retired.
    pub fn reset(&mut self) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "identity initialization still reserved"
        );
        self.reset_records()
    }

    fn reset_records(&self) -> Result<()> {
        let _trust_lock = self.directory.lock(c"trust.lock")?;
        if self.directory.read(c"identity.reset", 0)?.is_none() {
            let marker = self.directory.child(
                c"identity.reset",
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )?;
            marker.sync_all()?;
        }
        #[cfg(test)]
        tests::checkpoint("reset-intent-sync")?;
        self.directory.sync()?;
        self.erase_records()
    }

    fn erase_records(&self) -> Result<()> {
        for name in [
            c"trust.bin",
            c"trust.pending",
            c"trust.next",
            c"trust.reset",
            c"identity.bin",
            c"identity.pending",
        ] {
            self.directory.remove_if_present(name)?;
            #[cfg(test)]
            tests::checkpoint(name.to_str().unwrap())?;
        }
        self.directory.sync()?;
        #[cfg(test)]
        tests::checkpoint("reset-remove")?;
        self.directory.remove(c"identity.reset")?;
        #[cfg(test)]
        tests::checkpoint("reset-retirement-sync")?;
        self.directory.sync()
    }
}
