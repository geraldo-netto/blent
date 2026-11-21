//! T1428/T1509: entry owns the PID publication through workflow and exit drain.
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Publication {
    path: PathBuf,
    pid: u32,
}

impl Publication {
    pub(crate) fn new(path: PathBuf) -> io::Result<Self> {
        let pid = std::process::id();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        let mut pending = tempfile::NamedTempFile::new_in(parent.unwrap_or(Path::new(".")))?;
        pending
            .as_file_mut()
            .write_all(pid.to_string().as_bytes())?;
        pending.persist(&path).map_err(|error| error.error)?;
        Ok(Self { path, pid })
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        crate::remove_pid_file_if_ours(&self.path, self.pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1428_publication_removes_only_its_own_pid() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("daemon.pid");
        let publication = Publication::new(path.clone()).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            std::process::id().to_string()
        );
        drop(publication);
        assert!(!path.exists());
        let publication = Publication::new(path.clone()).unwrap();
        std::fs::write(&path, "0").unwrap();
        drop(publication);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "0");
    }

    #[test]
    fn t1428_failed_publication_and_unwind_leave_no_owned_pid() {
        let root = tempfile::tempdir().unwrap();
        assert!(Publication::new(root.path().join("missing/daemon.pid")).is_err());
        let path = root.path().join("daemon.pid");
        let result = std::panic::catch_unwind(|| {
            let _publication = Publication::new(path.clone()).unwrap();
            panic!("T1428: workflow unwinds after publication");
        });
        assert!(result.is_err());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn t1428_cancelled_workflow_retires_pid() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("daemon.pid");
        let owned_path = path.clone();
        let (ready, published) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _publication = Publication::new(owned_path).unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        published.await.unwrap();
        assert!(path.exists());
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!path.exists());
    }

    struct FileLimit {
        old: libc::rlimit,
        signal: libc::sighandler_t,
    }
    impl FileLimit {
        fn shorter_than_pid() -> Self {
            let mut old = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_FSIZE, &mut old) }, 0);
            let signal = unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) };
            assert_ne!(signal, libc::SIG_ERR);
            let limited = libc::rlimit {
                rlim_cur: (std::process::id().to_string().len() - 1) as libc::rlim_t,
                rlim_max: old.rlim_max,
            };
            assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &limited) }, 0);
            Self { old, signal }
        }
    }
    impl Drop for FileLimit {
        fn drop(&mut self) {
            assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &self.old) }, 0);
            unsafe {
                libc::signal(libc::SIGXFSZ, self.signal);
            }
        }
    }

    #[test]
    fn t1443_partial_publication_child() {
        let Some(root) = std::env::var_os("BLENT_T1443_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let path = root.join("daemon.pid");
        let original = std::fs::read(&path).ok();
        let limit = FileLimit::shorter_than_pid();
        let result = Publication::new(path.clone());
        drop(limit);
        let error = result.err().unwrap();
        assert_eq!(error.raw_os_error(), Some(libc::EFBIG), "{error:?}");
        assert_eq!(
            std::fs::read(&path).ok(),
            original,
            "T1443 failed write changed PID hint"
        );
        assert_eq!(
            std::fs::read_dir(root).unwrap().count(),
            usize::from(original.is_some()),
            "T1443 temporary publication leaked"
        );
    }

    #[test]
    fn t1443_partial_write_preserves_absent_or_previous_pid_hint() {
        for original in [None, Some("424242")] {
            let root = tempfile::tempdir().unwrap();
            if let Some(original) = original {
                std::fs::write(root.path().join("daemon.pid"), original).unwrap();
            }
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "daemon_pid::tests::t1443_partial_publication_child",
                    "--nocapture",
                ])
                .env("BLENT_T1443_ROOT", root.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn t1443_rejected_publication_preserves_destination_and_cleans_temporary_file() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("occupied");
        std::fs::create_dir(&destination).unwrap();
        assert!(Publication::new(destination.clone()).is_err());
        assert!(destination.is_dir());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
