//! T1114: native independent-description interleavings, joined before assertions.
use super::*;
use std::{io, os::fd::AsRawFd, sync::mpsc};

pub(super) fn private_root() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

fn replace(path: &Path, payload: &str) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(payload.as_bytes())?;
    file.persist(path)?;
    Ok(())
}

#[derive(Debug)]
enum Publisher {
    Blocked,
    Published,
}

fn foreign_publish(path: PathBuf, payload: String, entered: mpsc::Sender<Publisher>) -> Result<()> {
    let directory = std::fs::File::open(path.parent().unwrap())?;
    let fd = directory.as_raw_fd();
    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        replace(&path, &payload)?;
        entered.send(Publisher::Published)?;
        return Ok(());
    }
    let error = io::Error::last_os_error();
    anyhow::ensure!(
        error.kind() == io::ErrorKind::WouldBlock,
        "T1114 unexpected lock error: {error}"
    );
    entered.send(Publisher::Blocked)?;
    anyhow::ensure!(
        unsafe { libc::flock(fd, libc::LOCK_EX) } == 0,
        "T1114 acquire original directory"
    );
    replace(&path, &payload)
}

#[test]
fn t1114_snapshot_cleanup_cannot_unlink_a_foreign_publication() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    lease.publish(500).unwrap();
    let foreign_token = tempfile::tempfile_in(root.path()).unwrap();
    let payload = format!(
        "{} {} {} 1200 500\n",
        lease.identity.0,
        lease.identity.1,
        foreign_token.metadata().unwrap().ino()
    );
    let path = lease.path.clone();
    let expected = payload.clone();
    let (start, waiting) = mpsc::channel();
    let (entered, observed) = mpsc::channel();
    let outcome = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            waiting.recv()?;
            foreign_publish(path, payload, entered)
        });
        lease
            .clear_locked_with(move || {
                start.send(()).unwrap();
                // A finite fixture handshake, never a compiler/native-cleanup cap.
                observed.recv_timeout(Duration::from_secs(5)).unwrap();
            })
            .unwrap();
        writer.join().unwrap()
    });
    outcome.unwrap();
    assert_eq!(
        std::fs::read_to_string(&lease.path).unwrap(),
        expected,
        "T1114 original cleanup removed the successor's control"
    );
}

#[test]
fn t1114_original_directory_retirement_preserves_a_replacement_namespace() {
    use std::os::unix::fs::DirBuilderExt;
    let root = private_root();
    let active = root.path().join("active");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&active)
        .unwrap();
    let fifo = active.join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    lease.publish(500).unwrap();
    let retired = root.path().join("retired");
    std::fs::rename(&active, &retired).unwrap();
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&active)
        .unwrap();
    let replacement = active.join("capture.idle");
    std::fs::write(&replacement, "foreign replacement namespace").unwrap();
    lease.clear();
    assert!(
        !retired.join("capture.idle").exists(),
        "T1114 original directory lost its cleanup sponsor"
    );
    assert_eq!(
        std::fs::read_to_string(replacement).unwrap(),
        "foreign replacement namespace"
    );
}

#[test]
fn t1114_stop_during_native_directory_wait_cannot_publish_late() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    let directory = directory::Directory::open(root.path()).unwrap();
    let (entered, observed) = mpsc::channel();
    let (handshake, outcome) = std::thread::scope(|scope| {
        let held = directory.lock().unwrap();
        let writer = scope.spawn(|| {
            lease.publish_locked(500, || {
                entered.send(()).unwrap();
            })
        });
        let handshake = observed.recv_timeout(Duration::from_secs(5));
        lease.stop.revoke();
        drop(held);
        (handshake, writer.join().unwrap())
    });
    handshake.unwrap();
    assert!(outcome.is_err());
    assert!(!lease.path.exists());
}

#[test]
fn t1114_failed_persist_settles_temp_paths_before_original_directory_can_close() {
    let root = private_root();
    let fifo = root.path().join("capture.fifo");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let lease = Lease::open(&fifo, Default::default()).unwrap();
    std::fs::create_dir(&lease.path).unwrap();
    let before = std::fs::read_dir(root.path()).unwrap().count();
    let error = lease.publish(500).unwrap_err();
    assert!(
        error.downcast_ref::<tempfile::PersistError>().is_none(),
        "T1114 native directory borrow escaped through an owned TempPath"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
    assert!(lease.path.is_dir());
}

#[test]
fn t1114_directory_validation_and_bounded_name_views_fail_closed() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = private_root();
    let directory = directory::Directory::open(root.path()).unwrap();
    for path in ["", "/", ".", "..", "a/.."] {
        assert!(directory.path(Path::new(path)).is_err());
    }
    for path in ["slot.idle", "../slot.idle", "/other/slot.idle"] {
        assert_eq!(
            directory.path(Path::new(path)).unwrap(),
            directory.root().join("slot.idle")
        );
    }
    let info = std::fs::metadata(root.path()).unwrap();
    assert!(directory::validate(&info, info.uid() + 1).is_err());
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    let rejected = directory::Directory::open(root.path()).is_err();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(rejected);
    let link = root.path().join("alias");
    symlink(root.path(), &link).unwrap();
    assert!(directory::Directory::open(&link).is_err());
    for fd in -256..0 {
        assert!(directory::lock_fd(fd, libc::LOCK_EX | libc::LOCK_NB).is_err());
    }
}

async fn native_lock_probe(
    scope: &blent::native_scope::Scope,
    root: &Path,
) -> std::io::Result<std::process::Output> {
    let script = "import errno,fcntl,os,sys\nf=os.open(sys.argv[1],os.O_RDONLY|os.O_DIRECTORY)\ntry:\n fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)\n print('free')\nexcept OSError as e:\n if e.errno not in (errno.EACCES,errno.EAGAIN): raise\n print('busy')\nfinally:\n os.close(f)\n";
    let mut command = tokio::process::Command::new("python3");
    command.arg("-c").arg(script).arg(root);
    scope
        .output(&mut command, None, Duration::from_secs(2))
        .await
}

#[tokio::test]
async fn t1114_directory_guard_coordinates_real_processes_until_original_unlock() {
    let root = private_root();
    let directory = directory::Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let blocked = native_lock_probe(&scope, root.path()).await;
    drop(held);
    let released = native_lock_probe(&scope, root.path()).await;
    drop(scope);
    while !retired.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let blocked = blocked.unwrap();
    let released = released.unwrap();
    assert!(blocked.status.success());
    assert!(released.status.success());
    assert_eq!(blocked.stdout, b"busy\n");
    assert_eq!(released.stdout, b"free\n");
}
