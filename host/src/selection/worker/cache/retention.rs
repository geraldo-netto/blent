//! Linux filesystem retention adapter. The dedicated tuning-cache directory owns
//! its regular 64-lowercase-hex.json namespace. Other names/symlinks are untouched.
//! One nonblocking file lock serializes writers; a bounded heap retains only the
//! newest capacity-1 other records. The caller reserves its current name through
//! atomic publication. Read handles keep their original inodes after eviction.
use anyhow::{Context, Result};
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    ffi::OsStr,
    fs,
    io::Write,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(super) struct Transaction {
    directory: fs::File,
    _lock: fs::File,
}

impl Transaction {
    pub(super) fn open(directory: &Path) -> Result<Self> {
        let directory_file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(directory)?;
        let pinned = descriptor_path(&directory_file);
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(pinned.join("retention.lock"))?;
        anyhow::ensure!(
            lock.metadata()?.is_file(),
            "cache lock is not a regular file"
        );
        lock.try_lock().context("tuning cache writer is busy")?;
        Ok(Self {
            directory: directory_file,
            _lock: lock,
        })
    }

    pub(super) fn publish(&self, name: &OsStr, data: &[u8], capacity: u32) -> Result<()> {
        anyhow::ensure!(
            Path::new(name).file_name() == Some(name),
            "invalid cache record name"
        );
        anyhow::ensure!(
            data.len() as u64 <= super::MAX_BYTES,
            "profile cache too large"
        );
        let directory = descriptor_path(&self.directory);
        let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
        temporary.write_all(data)?;
        temporary.as_file().sync_all()?;
        self.reserve(name, capacity)?;
        temporary.persist(directory.join(name)).map_err(|failure| {
            // PersistError owns the tempfile; retire it while its directory fd
            // is still pinned, never return that path-owning error to the caller.
            drop(failure.file);
            failure.error
        })?;
        Ok(())
    }

    pub(super) fn reserve(&self, current: &OsStr, capacity: u32) -> Result<()> {
        anyhow::ensure!(
            (1..=blent_config::MAX_TUNING_CACHE_ENTRIES).contains(&capacity),
            "invalid tuning cache capacity"
        );

        // Repeat after deletions: read_dir is not a snapshot during mutation.
        // With writers excluded, each nonfinal pass retires at least one name.
        while self.pass(current, capacity as usize)? {}
        Ok(())
    }

    fn pass(&self, current: &OsStr, capacity: usize) -> Result<bool> {
        let mut retained = BinaryHeap::new();
        let mut removed = false;
        let directory = descriptor_path(&self.directory);
        for entry in fs::read_dir(&directory)? {
            let Some(record) = candidate(entry?, current)? else {
                continue;
            };
            retained.push(Reverse(record));
            if retained.len() >= capacity {
                let Reverse((_, name)) = retained.pop().unwrap();
                remove(&directory.join(name))?;
                removed = true;
            }
        }
        Ok(removed)
    }
}

fn descriptor_path(directory: &fs::File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()))
}

fn record_name(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        let Some(stem) = name.strip_suffix(".json") else {
            return false;
        };
        stem.len() == 64
            && stem
                .bytes()
                .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
    })
}

fn candidate(
    entry: fs::DirEntry,
    current: &OsStr,
) -> Result<Option<(SystemTime, std::ffi::OsString)>> {
    let name = entry.file_name();
    if name == current || !record_name(&name) {
        return Ok(None);
    }
    let metadata = match entry.metadata() {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Ok(None);
    }
    Ok(Some((metadata.modified()?, name)))
}

fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
