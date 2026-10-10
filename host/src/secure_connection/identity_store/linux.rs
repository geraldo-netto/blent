//! Linux private persistence adapter. The broker supplies an existing private
//! directory; all child operations use its pinned descriptor and fixed names.
//! No path selection, user-directory creation or Windows support is implied.
use super::{IdentityStorage, MAX_RECORD};
use anyhow::{ensure, Context, Result};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub struct LinuxIdentityStore {
    directory: File,
    _lock: File,
    pending: Option<File>,
}
impl LinuxIdentityStore {
    /// Keep this owner alive while consuming its identity. Fail immediately if
    /// another process owns the store. Never unlink/replace the stable lock.
    pub fn open(path: &Path) -> Result<Self> {
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        private(&directory, true)?;
        let lock = child(&directory, c"identity.lock", libc::O_RDWR | libc::O_CREAT)?;
        private(&lock, false)?;
        lock.try_lock().context("identity store already owned")?;
        Ok(Self {
            directory,
            _lock: lock,
            pending: None,
        })
    }
    fn read_identity(&self) -> Result<Option<Vec<u8>>> {
        let file = match child(&self.directory, c"identity.bin", libc::O_RDONLY) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        private(&file, false)?;
        let mut bytes = Vec::new();
        file.take((MAX_RECORD + 1) as u64).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= MAX_RECORD, "identity record exceeds limit");
        Ok(Some(bytes))
    }
    fn no_pending(&self) -> Result<()> {
        match child(&self.directory, c"identity.pending", libc::O_RDONLY) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => {
                anyhow::bail!("interrupted identity initialization; preserve storage for recovery")
            }
        }
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
        let file = child(
            &self.directory,
            c"identity.pending",
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
        )?;
        private(&file, false)?;
        #[cfg(test)]
        tests::checkpoint("pending-sync")?;
        file.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("reserve-sync")?;
        self.directory.sync_all()?;
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
        let result = unsafe {
            libc::renameat2(
                self.directory.as_raw_fd(),
                c"identity.pending".as_ptr(),
                self.directory.as_raw_fd(),
                c"identity.bin".as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        ensure!(
            result == 0,
            "identity publication failed: {}",
            std::io::Error::last_os_error()
        );
        #[cfg(test)]
        tests::checkpoint("publish-sync")?;
        self.directory.sync_all()?;
        #[cfg(test)]
        tests::checkpoint("published")?;
        Ok(())
    }
}

fn child(directory: &File, name: &CStr, flags: i32) -> std::io::Result<File> {
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn private(file: &File, directory: bool) -> Result<()> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "identity storage has different owner"
    );
    if directory {
        ensure!(
            metadata.is_dir() && metadata.mode() & 0o777 == 0o700,
            "identity directory is not private"
        );
    } else {
        ensure!(
            metadata.is_file() && metadata.nlink() == 1 && metadata.mode() & 0o777 == 0o600,
            "identity file is not private and exclusive"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
