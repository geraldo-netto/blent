//! Process-lifetime advisory digests, never native handles or trust authority.
//! At most 256 canonical paths, stat records and 32-byte digests are retained.
//! Files remain owned only by the current hash call. Local filesystem metadata
//! coherence is required; every miss checks the original descriptor after reading.
use std::{
    collections::VecDeque,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const MAX_ENTRIES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Stamp {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl From<&std::fs::Metadata> for Stamp {
    fn from(value: &std::fs::Metadata) -> Self {
        Self {
            device: value.dev(),
            inode: value.ino(),
            bytes: value.len(),
            mode: value.mode(),
            modified: (value.mtime(), value.mtime_nsec()),
            changed: (value.ctime(), value.ctime_nsec()),
        }
    }
}
struct Entry {
    path: PathBuf,
    stamp: Stamp,
    digest: [u8; 32],
}
#[derive(Default)]
pub(super) struct Memo(VecDeque<Entry>);

pub(super) fn cache() -> &'static Mutex<Memo> {
    static CACHE: OnceLock<Mutex<Memo>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}
pub(super) fn capacity() -> Option<usize> {
    let value = match std::env::var("BLENT_RUNTIME_HASH_CACHE_ENTRIES") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Some(64),
        Err(_) => return None,
    };
    let value: usize = value.parse().ok()?;
    (value <= MAX_ENTRIES).then_some(value)
}
impl Memo {
    fn resize(&mut self, limit: usize) {
        assert!(limit <= MAX_ENTRIES);
        while self.0.len() > limit {
            self.0.pop_front();
        }
        if limit == 0 {
            self.0.shrink_to_fit();
        }
    }
    pub(super) fn lookup(&mut self, path: &Path, stamp: Stamp, limit: usize) -> Option<[u8; 32]> {
        self.resize(limit);
        let index = self.0.iter().position(|entry| entry.path == path)?;
        let entry = self.0.remove(index)?;
        if entry.stamp != stamp {
            return None;
        }
        let digest = entry.digest;
        self.0.push_back(entry);
        Some(digest)
    }
    pub(super) fn remember(&mut self, path: PathBuf, stamp: Stamp, digest: [u8; 32], limit: usize) {
        self.resize(limit);
        self.0.retain(|entry| entry.path != path);
        if limit == 0 {
            return;
        }
        self.resize(limit - 1);
        self.0.push_back(Entry {
            path,
            stamp,
            digest,
        });
    }
}

#[cfg(test)]
mod tests;
