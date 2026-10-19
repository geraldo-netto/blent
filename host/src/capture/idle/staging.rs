//! T1118: one original staging file stays inside its native directory borrow.
use super::directory::{self, Guard};
use anyhow::{ensure, Context, Result};
use std::{
    fs::{File, OpenOptions},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    time::Duration,
};
use tempfile::NamedTempFile;

pub(super) const PREFIX: &str = ".blent-idle-stage-";

/// Pre-admission only: the caller just locked a freshly opened description and
/// has not allocated a local staging owner under it. Other owners cannot enter.
pub(super) fn collect(directory: &Guard<'_>) -> Result<()> {
    let mut examined = 0u64;
    loop {
        let mut found = false;
        for entry in std::fs::read_dir(directory.root())? {
            let entry = entry?;
            examined = examined
                .checked_add(1)
                .context("idle namespace scan exhausted")?;
            if examined.is_multiple_of(512) {
                tracing::debug!(examined, "Scanning original idle namespace");
            }
            if !entry
                .file_name()
                .as_encoded_bytes()
                .starts_with(PREFIX.as_bytes())
            {
                continue;
            }
            remove_abandoned(&entry.path())?;
            found = true;
        }
        if !found {
            return Ok(()); // A final full pass verifies no reserved stage remains.
        }
    }
}

fn remove_abandoned(path: &Path) -> Result<()> {
    // Every current live staging owner holds the same exclusive directory lock.
    // Its release/owner loss is the admission fence, never mtime or a PID guess.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let info = file.metadata()?;
    ensure!(
        info.file_type().is_file(),
        "reserved idle stage is not a regular file"
    );
    directory::validate_private(&info, unsafe { libc::geteuid() })?;
    remove_original(&file, path, |path| std::fs::remove_file(path))
}

pub(super) struct Staging<'a> {
    file: Option<NamedTempFile>,
    directory: &'a Guard<'a>,
}

impl<'a> Staging<'a> {
    pub(super) fn create(directory: &'a Guard<'a>) -> io::Result<Self> {
        let file = tempfile::Builder::new()
            .prefix(PREFIX)
            .tempfile_in(directory.root())?;
        Ok(Self {
            file: Some(file),
            directory,
        })
    }

    pub(super) fn writer(&mut self) -> &mut dyn io::Write {
        self.file
            .as_mut()
            .expect("active staging owner")
            .as_file_mut()
    }

    pub(super) fn persist(mut self, target: &Path) -> io::Result<File> {
        let target = self.directory.path(target)?;
        match self
            .file
            .take()
            .expect("active staging owner")
            .persist(target)
        {
            Ok(file) => Ok(file),
            Err(failure) => {
                self.file = Some(failure.file);
                Err(failure.error)
            }
        }
    }
}

pub(super) fn discard(
    file: NamedTempFile,
    _directory: &Guard<'_>,
    mut remove: impl FnMut(&Path) -> io::Result<()>,
) {
    let (file, mut path) = file.into_parts();
    // This owner settles the exact original entry. A later TempPath destructor
    // must neither retry a reused pathname nor perform unobserved I/O.
    path.disable_cleanup(true);
    super::retire_request(
        || remove_original(&file, &path, &mut remove),
        || std::thread::sleep(Duration::from_millis(100)),
    );
    drop(path);
    drop(file);
}

fn remove_original(
    file: &File,
    path: &Path,
    remove: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<()> {
    let current = match std::fs::symlink_metadata(path) {
        Ok(info) => info,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let original = file.metadata()?;
    if (current.dev(), current.ino()) != (original.dev(), original.ino()) {
        return Ok(());
    }
    directory::remove_owned(path, remove)
        .with_context(|| format!("remove original staging file {path:?}"))
}

impl Drop for Staging<'_> {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            discard(file, self.directory, |path| std::fs::remove_file(path));
        }
    }
}

#[cfg(test)]
mod tests;
