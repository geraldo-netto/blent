//! Linux private request namespace. A pinned original directory owns every effect.
use anyhow::{ensure, Result};
use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

pub(super) struct Directory(File);
pub(super) struct Guard<'a>(&'a File);

impl Directory {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        validate(&file.metadata()?, unsafe { libc::geteuid() })?;
        Ok(Self(file))
    }

    pub(super) fn root(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.0.as_raw_fd()))
    }

    pub(super) fn path(&self, path: &Path) -> io::Result<PathBuf> {
        let name = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "idle effect needs a file name")
        })?;
        Ok(self.root().join(name))
    }

    pub(super) fn lock(&self) -> io::Result<Guard<'_>> {
        lock_fd(self.0.as_raw_fd(), libc::LOCK_EX)?;
        Ok(Guard(&self.0))
    }
}

pub(super) fn validate(info: &Metadata, uid: u32) -> Result<()> {
    ensure!(info.uid() == uid, "idle directory has a foreign owner");
    ensure!(
        info.mode() & 0o022 == 0,
        "idle directory is writable by others"
    );
    Ok(())
}

pub(super) fn lock_fd(fd: RawFd, operation: i32) -> io::Result<()> {
    if unsafe { libc::flock(fd, operation) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        // A failed unlock never authorizes closing/reusing the original File.
        // Its lease still owns that descriptor through actual native cleanup.
        let _ = lock_fd(self.0.as_raw_fd(), libc::LOCK_UN);
    }
}
