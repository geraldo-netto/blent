//! One daemon per validated runtime directory; entry retains the original file
//! description until native exit drain finishes. Never unlink the lock inode.
use anyhow::{Context, Result};
use std::fs::{File, OpenOptions, TryLockError};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(crate) struct Lease {
    _file: File,
}
impl Lease {
    pub(crate) fn acquire(directory: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(directory.join("daemon.lock"))
            .context("open daemon startup lock")?;
        anyhow::ensure!(
            file.metadata()?.is_file(),
            "daemon lock must be a regular file"
        );
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => {
                anyhow::bail!("blent daemon already running; startup lock is owned")
            }
            Err(TryLockError::Error(error)) => Err(error).context("lock daemon startup"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn t1695_simultaneous_requests_keep_one_original_lease_until_drop() {
        let root = tempfile::tempdir().unwrap();
        let barrier = Barrier::new(3);
        let results = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                barrier.wait();
                Lease::acquire(root.path())
            });
            let second = scope.spawn(|| {
                barrier.wait();
                Lease::acquire(root.path())
            });
            barrier.wait();
            [first.join().unwrap(), second.join().unwrap()]
        });
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert!(Lease::acquire(root.path()).is_err());
        drop(results);
        let next = Lease::acquire(root.path()).unwrap();
        drop(next);
        assert!(root.path().join("daemon.lock").is_file());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn t1695_invalid_lock_objects_reject_without_replacing_them() {
        let root = tempfile::tempdir().unwrap();
        let lock = root.path().join("daemon.lock");
        assert!(Lease::acquire(&root.path().join("missing")).is_err());
        std::fs::create_dir(&lock).unwrap();
        assert!(Lease::acquire(root.path()).is_err());
        assert!(lock.is_dir());
        std::fs::remove_dir(&lock).unwrap();
        std::os::unix::fs::symlink("absent", &lock).unwrap();
        assert!(Lease::acquire(root.path()).is_err());
        assert!(lock.is_symlink());
        std::fs::remove_file(&lock).unwrap();
        let path = std::ffi::CString::new(lock.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        assert!(Lease::acquire(root.path()).is_err());
    }
}
