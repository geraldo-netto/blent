//! A partial raw frame retires its FIFO inode, not merely its write descriptor.
use anyhow::{Context, Result};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Identity {
    device: u64,
    inode: u64,
}

impl Identity {
    pub(super) fn from_reset_line(line: &str) -> Option<Self> {
        let mut words = line.strip_prefix("FIFO_RESET ")?.split_whitespace();
        let id = Self {
            device: words.next()?.parse().ok()?,
            inode: words.next()?.parse().ok()?,
        };
        words.next().is_none().then_some(id)
    }

    fn at(path: &Path) -> Result<Self> {
        let meta = std::fs::symlink_metadata(path).context("stat capture FIFO")?;
        Self::from_metadata(&meta)
    }

    fn from_metadata(meta: &std::fs::Metadata) -> Result<Self> {
        anyhow::ensure!(meta.file_type().is_fifo(), "capture path is not a FIFO");
        Ok(Self {
            device: meta.dev(),
            inode: meta.ino(),
        })
    }

    pub(super) fn matches(self, path: &Path) -> Result<bool> {
        Ok(Self::at(path)? == self)
    }
}

/// Own a particular inode, not a slot name. O_PATH pins identity without opening
/// a reader/writer endpoint or affecting FIFO connection and EOF semantics.
#[derive(Clone)]
pub(super) struct Owned(std::sync::Arc<Inode>);
struct Inode {
    path: PathBuf,
    identity: Identity,
    _inode: std::fs::File,
}

impl Owned {
    pub(super) fn rotate(&mut self) -> Result<()> {
        let retired = self.0.identity;
        self.replace_retired(retired)?;
        anyhow::ensure!(
            self.0.identity != retired,
            "capture FIFO ownership changed before restart"
        );
        Ok(())
    }
    pub(super) fn create(path: &Path) -> Result<Self> {
        super::helper::ensure_fifo(path)?;
        let inode = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(path)
            .context("pin owned capture FIFO")?;
        let identity = Identity::from_metadata(&inode.metadata()?)?;
        Ok(Self(std::sync::Arc::new(Inode {
            path: path.into(),
            identity,
            _inode: inode,
        })))
    }

    /// Caller has retired the old reader. Allocate before replacing the FIFO
    /// so an old writer/reset report cannot authorize writes to the fresh one.
    pub(super) fn replace_retired(&mut self, retired: Identity) -> Result<()> {
        if retired != self.0.identity || !retired.matches(&self.0.path)? {
            return Ok(());
        }
        let parent = self.0.path.parent().context("capture FIFO has no parent")?;
        let staging = tempfile::Builder::new()
            .prefix("fifo-reset-")
            .tempdir_in(parent)?;
        let mut replacement = Self::create(&staging.path().join("frames"))?;
        anyhow::ensure!(
            replacement.0.identity != retired,
            "replacement reused the retired FIFO"
        );
        if retired.matches(&self.0.path)? {
            std::fs::rename(&replacement.0.path, &self.0.path)
                .context("replace retired capture FIFO")?;
            std::sync::Arc::get_mut(&mut replacement.0).unwrap().path = self.0.path.clone();
            *self = replacement;
        }
        Ok(())
    }
}

impl Drop for Inode {
    fn drop(&mut self) {
        if self.identity.matches(&self.path).unwrap_or(false) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t814_fifo_lease_preserves_inode_until_final_owner_retires() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("frames");
        let owner = Owned::create(&path).unwrap();
        let lease = owner.clone();
        drop(owner);
        assert!(lease.0.identity.matches(&path).unwrap());
        drop(lease);
        assert!(!path.exists());

        let mut owner = Owned::create(&path).unwrap();
        let retired = owner.clone();
        owner.rotate().unwrap();
        assert_ne!(retired.0.identity, owner.0.identity);
        drop(retired);
        assert!(owner.0.identity.matches(&path).unwrap());
        drop(owner);
        assert!(!path.exists());
    }

    #[test]
    fn t498_rotation_preserves_replacements_and_rejects_missing_ownership() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("frames");
        let mut owned = Owned::create(&path).unwrap();
        let retired = owned.0.identity;
        owned.rotate().unwrap();
        let fresh = owned.0.identity;
        owned.replace_retired(retired).unwrap();
        assert_eq!(
            owned.0.identity, fresh,
            "T498: late reset rotated fresh FIFO"
        );
        std::fs::remove_file(&path).unwrap();
        assert!(owned.rotate().is_err());
        let replacement = Owned::create(&path).unwrap();
        assert!(owned.rotate().is_err());
        drop(owned);
        assert!(replacement.0.identity.matches(&path).unwrap());
    }

    #[test]
    fn t498_reset_parser_bounds_and_mutation_corpus() {
        for a in ["0", "1", "18446744073709551615"] {
            for b in ["0", "1", "18446744073709551615"] {
                assert!(Identity::from_reset_line(&format!("FIFO_RESET {a} {b}")).is_some());
            }
        }
        for bad in ["", "-1", "18446744073709551616", "1.0", "NaN", "\0"] {
            assert!(Identity::from_reset_line(&format!("FIFO_RESET {bad} 1")).is_none());
            assert!(Identity::from_reset_line(&format!("FIFO_RESET 1 {bad}")).is_none());
        }
        let seed = b"FIFO_RESET 123 456";
        for offset in 0..seed.len() {
            for byte in 0..=255 {
                let mut input = seed.to_vec();
                input[offset] = byte;
                if let Ok(text) = std::str::from_utf8(&input) {
                    let _ = Identity::from_reset_line(text);
                }
            }
        }
        assert!(Identity::from_reset_line("FIFO_RESET 1 2 extra").is_none());
    }
}
