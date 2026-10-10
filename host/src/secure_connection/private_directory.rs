//! Shared Linux private-record primitives for identity and peer-trust adapters.
//! Fixed child names and a pinned descriptor avoid path replacement during I/O.
use anyhow::{ensure, Context, Result};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(super) struct Directory {
    file: File,
    _lock: File,
}
impl Directory {
    pub(super) fn open(path: &Path, lock_name: &CStr) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        private(&file, true)?;
        let lock = child(&file, lock_name, libc::O_RDWR | libc::O_CREAT)?;
        private(&lock, false)?;
        lock.try_lock()
            .context("private record store already owned")?;
        Ok(Self { file, _lock: lock })
    }
    pub(super) fn child(&self, name: &CStr, flags: i32) -> Result<File> {
        let file = child(&self.file, name, flags)?;
        private(&file, false)?;
        Ok(file)
    }
    pub(super) fn read(&self, name: &CStr, maximum: usize) -> Result<Option<Vec<u8>>> {
        let file = match child(&self.file, name, libc::O_RDONLY) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        private(&file, false)?;
        let mut bytes = Vec::new();
        file.take((maximum + 1) as u64).read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= maximum, "private record exceeds limit");
        Ok(Some(bytes))
    }
    pub(super) fn ensure_absent(&self, name: &CStr) -> Result<()> {
        match child(&self.file, name, libc::O_RDONLY) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => anyhow::bail!(
                "interrupted private record transaction; preserve storage for recovery"
            ),
        }
    }
    pub(super) fn sync(&self) -> Result<()> {
        Ok(self.file.sync_all()?)
    }
    pub(super) fn rename(&self, from: &CStr, to: &CStr, replace: bool) -> Result<()> {
        let flags = if replace { 0 } else { libc::RENAME_NOREPLACE };
        let result = unsafe {
            libc::renameat2(
                self.file.as_raw_fd(),
                from.as_ptr(),
                self.file.as_raw_fd(),
                to.as_ptr(),
                flags,
            )
        };
        ensure!(
            result == 0,
            "private record publication failed: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }
    pub(super) fn remove(&self, name: &CStr) -> Result<()> {
        let result = unsafe { libc::unlinkat(self.file.as_raw_fd(), name.as_ptr(), 0) };
        ensure!(
            result == 0,
            "private record retirement failed: {}",
            std::io::Error::last_os_error()
        );
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
        "private storage has different owner"
    );
    if directory {
        ensure!(
            metadata.is_dir() && metadata.mode() & 0o777 == 0o700,
            "record directory is not private"
        );
    } else {
        ensure!(
            metadata.is_file() && metadata.nlink() == 1 && metadata.mode() & 0o777 == 0o600,
            "record file is not private and exclusive"
        );
    }
    Ok(())
}
