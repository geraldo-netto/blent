//! T1428: the daemon workflow owns its PID publication through every return path.
use std::{io, path::PathBuf};

pub(crate) struct Publication {
    path: PathBuf,
    pid: u32,
}

impl Publication {
    pub(crate) fn new(path: PathBuf) -> io::Result<Self> {
        let pid = std::process::id();
        std::fs::write(&path, pid.to_string())?;
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
}
