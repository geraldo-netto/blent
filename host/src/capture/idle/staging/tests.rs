use super::*;
use crate::capture::idle::{directory::Directory, lease_tests::private_root};

#[test]
fn t1118_staging_unlink_errors_cannot_be_ignored_by_destructor_cleanup() {
    use std::{os::fd::AsRawFd, os::unix::fs::MetadataExt};
    let root = private_root();
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    let file = tempfile::NamedTempFile::new_in(held.root()).unwrap();
    let path = file.path().to_owned();
    let descriptor = file.as_file().as_raw_fd();
    let original = file.as_file().metadata().unwrap();
    let mut attempts = 0;
    discard(file, &held, |path| {
        let retained = std::fs::metadata(format!("/proc/self/fd/{descriptor}")).unwrap();
        assert_eq!(
            (retained.dev(), retained.ino()),
            (original.dev(), original.ino())
        );
        attempts += 1;
        if attempts < 3 {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            std::fs::remove_file(path)
        }
    });
    assert_eq!(
        attempts, 3,
        "T1118 failed native deletion was not observed/settled"
    );
    assert!(!path.exists());
}

#[test]
fn t1118_owner_loss_child() {
    let Ok(root) = std::env::var("BLENT_T1118_STAGE_OWNER_LOSS") else {
        return;
    };
    let directory = Directory::open(Path::new(&root)).unwrap();
    let held = directory.lock().unwrap();
    let mut file = Staging::create(&held).unwrap();
    file.writer()
        .write_all(b"owner-lost staging metadata")
        .unwrap();
    std::process::exit(0); // Deliberately bypass Rust stack destructors.
}

#[tokio::test]
async fn t1118_actual_process_loss_leaves_only_guarded_recoverable_debt() {
    let root = private_root();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "capture::idle::staging::tests::t1118_owner_loss_child",
            "--nocapture",
        ])
        .env("BLENT_T1118_STAGE_OWNER_LOSS", root.path());
    let result = scope
        .output(&mut command, None, std::time::Duration::from_secs(5))
        .await;
    drop(scope);
    while !retired.is_complete() {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    let result = result.unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    collect(&held).unwrap();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn t1118_staging_retirement_preserves_a_foreign_replacement() {
    let root = private_root();
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    let file = tempfile::NamedTempFile::new_in(held.root()).unwrap();
    let path = file.path().to_owned();
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"foreign staging replacement").unwrap();
    discard(file, &held, |path| std::fs::remove_file(path));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"foreign staging replacement"
    );
}

#[test]
fn t1118_next_guarded_admission_collects_owner_lost_stages_only() {
    let root = private_root();
    for _ in 0..4 {
        let (file, _) = tempfile::Builder::new()
            .prefix(PREFIX)
            .tempfile_in(root.path())
            .unwrap()
            .keep()
            .unwrap();
        drop(file); // Model an owner lost after creating a named native inode.
    }
    let other = root.path().join("unrelated-private-state");
    std::fs::write(&other, b"preserve unrelated state").unwrap();
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    collect(&held).unwrap();
    assert_eq!(
        std::fs::read_dir(root.path()).unwrap().count(),
        1,
        "T1118 abandoned staging debt survived next native admission"
    );
    assert_eq!(std::fs::read(other).unwrap(), b"preserve unrelated state");
}

#[test]
fn t1118_persist_and_invalid_target_settle_before_directory_borrow_ends() {
    let root = private_root();
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    let mut file = Staging::create(&held).unwrap();
    file.writer().write_all(b"bounded record").unwrap();
    let target = root.path().join("published");
    drop(file.persist(&target).unwrap());
    assert_eq!(std::fs::read(&target).unwrap(), b"bounded record");
    let file = Staging::create(&held).unwrap();
    assert!(file.persist(Path::new("/")).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn t1118_removed_original_entry_does_not_trigger_a_second_native_delete() {
    let root = private_root();
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    let file = tempfile::NamedTempFile::new_in(held.root()).unwrap();
    std::fs::remove_file(file.path()).unwrap();
    discard(file, &held, |_| {
        panic!("T1118 already removed original was deleted twice")
    });
    let file = tempfile::NamedTempFile::new_in(held.root()).unwrap();
    discard(file, &held, |path| {
        std::fs::remove_file(path)?;
        Err(io::Error::from(io::ErrorKind::NotFound))
    });
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn t1118_streamed_collection_preserves_mixed_namespace_entries() {
    use std::os::unix::fs::OpenOptionsExt;
    let root = private_root();
    for index in 0..257 {
        let pending = root.path().join(format!("{PREFIX}{index}"));
        drop(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(pending)
                .unwrap(),
        );
        std::fs::write(
            root.path().join(format!("other-state-{index}")),
            b"preserve",
        )
        .unwrap();
    }
    let directory = Directory::open(root.path()).unwrap();
    let held = directory.lock().unwrap();
    collect(&held).unwrap();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 257);
    for index in 0..257 {
        assert_eq!(
            std::fs::read(root.path().join(format!("other-state-{index}"))).unwrap(),
            b"preserve"
        );
    }
}

#[test]
fn t1118_untrusted_reserved_objects_reject_new_admission_and_are_preserved() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    for kind in 0..3 {
        let root = private_root();
        let reserved = root.path().join(format!("{PREFIX}untrusted"));
        let target = root.path().join("unrelated-target");
        std::fs::write(&target, b"preserve target").unwrap();
        match kind {
            0 => std::fs::create_dir(&reserved).unwrap(),
            1 => symlink(&target, &reserved).unwrap(),
            _ => {
                std::fs::write(&reserved, b"untrusted permissions").unwrap();
                std::fs::set_permissions(&reserved, std::fs::Permissions::from_mode(0o666))
                    .unwrap();
            }
        }
        let directory = Directory::open(root.path()).unwrap();
        let held = directory.lock().unwrap();
        assert!(collect(&held).is_err());
        assert!(std::fs::symlink_metadata(&reserved).is_ok());
        assert_eq!(std::fs::read(&target).unwrap(), b"preserve target");
        drop(held);
        let fifo = root.path().join("capture.fifo");
        crate::capture::helper::ensure_fifo(&fifo).unwrap();
        assert!(crate::capture::idle::Lease::open(&fifo, Default::default()).is_err());
    }
}
