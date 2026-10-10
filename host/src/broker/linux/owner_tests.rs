use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}

#[test]
fn t874_owner_election_keeps_stable_lock_and_only_removes_own_socket() {
    let root = root();
    let owner = Owner::bind(root.path(), 4).unwrap();
    let lock = std::fs::metadata(root.path().join("broker.lock")).unwrap();
    assert!(Owner::bind(root.path(), 4).is_err());
    assert!(owner.accept(unsafe { libc::geteuid() }).unwrap().is_none());
    let client =
        Socket::connect(&root.path().join("broker.sock"), unsafe { libc::geteuid() }).unwrap();
    let (peer, caller) = owner.accept(unsafe { libc::geteuid() }).unwrap().unwrap();
    assert_eq!(caller.pid, std::process::id());
    drop(owner);
    assert!(!root.path().join("broker.sock").exists());
    let replacement = Owner::bind(root.path(), 4).unwrap();
    assert_eq!(
        std::fs::metadata(root.path().join("broker.lock"))
            .unwrap()
            .ino(),
        lock.ino()
    );
    client.send(b"alive", &[]).unwrap();
    assert_eq!(peer.receive(&mut [0; 16]).unwrap().unwrap().length, 5);
    drop(replacement);
}

#[test]
fn t874_stale_socket_cleanup_requires_election_and_preserves_foreign_nodes() {
    let root = root();
    let endpoint = root.path().join("broker.sock");
    drop(Listener::bind(&endpoint, 1).unwrap());
    let owner = Owner::bind(root.path(), 1).unwrap();
    assert!(endpoint.exists());
    std::fs::remove_file(&endpoint).unwrap();
    std::fs::write(&endpoint, b"replacement").unwrap();
    drop(owner);
    assert_eq!(std::fs::read(&endpoint).unwrap(), b"replacement");
    assert!(Owner::bind(root.path(), 1).is_err());
    std::fs::remove_file(&endpoint).unwrap();
    symlink("broker.lock", &endpoint).unwrap();
    assert!(Owner::bind(root.path(), 1).is_err());
    assert!(std::fs::symlink_metadata(&endpoint)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn t874_renamed_directory_stays_pinned_and_drop_preserves_replacement_socket() {
    let root = root();
    let selected = root.path().join("selected");
    let moved = root.path().join("moved");
    std::fs::create_dir(&selected).unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o700)).unwrap();
    let owner = Owner::bind(&selected, 1).unwrap();
    std::fs::rename(&selected, &moved).unwrap();
    std::fs::create_dir(&selected).unwrap();
    std::fs::set_permissions(&selected, std::fs::Permissions::from_mode(0o700)).unwrap();
    let replacement = Owner::bind(&selected, 1).unwrap();
    drop(owner);
    assert!(!moved.join("broker.sock").exists());
    assert!(selected.join("broker.sock").exists());
    std::fs::remove_file(selected.join("broker.sock")).unwrap();
    let external = Listener::bind(&selected.join("broker.sock"), 1).unwrap();
    drop(replacement);
    assert!(selected.join("broker.sock").exists());
    drop(external);
}

#[test]
fn t874_private_directory_and_backlog_bounds_reject_without_cleanup() {
    let root = root();
    let endpoint = root.path().join("broker.sock");
    drop(Listener::bind(&endpoint, 1).unwrap());
    for backlog in [0, 1025, u32::MAX] {
        assert!(Owner::bind(root.path(), backlog).is_err());
        assert!(endpoint.exists());
    }
    for mode in [0o755, 0o770, 0o777, 0o1700] {
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(mode)).unwrap();
        assert!(Owner::bind(root.path(), 1).is_err());
    }
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let link = root.path().join("link");
    symlink(root.path(), &link).unwrap();
    assert!(Owner::bind(&link, 1).is_err());
    assert!(Owner::bind(Path::new("relative"), 1).is_err());
    assert!(Owner::bind(&root.path().join("missing"), 1).is_err());
}

fn bad_lock(mode: u32, hard_link: bool) {
    let root = root();
    let lock = root.path().join("broker.lock");
    std::fs::write(&lock, b"preserve").unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(mode)).unwrap();
    if hard_link {
        std::fs::hard_link(&lock, root.path().join("alias")).unwrap();
    }
    assert!(Owner::bind(root.path(), 1).is_err());
    assert_eq!(std::fs::read(&lock).unwrap(), b"preserve");
}

#[test]
fn t874_lock_symlink_hardlink_fifo_and_permissions_are_rejected() {
    bad_lock(0o644, false);
    bad_lock(0o600, true);
    let root = root();
    let lock = root.path().join("broker.lock");
    symlink("missing", &lock).unwrap();
    assert!(Owner::bind(root.path(), 1).is_err());
    std::fs::remove_file(&lock).unwrap();
    let directory = OpenOptions::new().read(true).open(root.path()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifoat(directory.as_raw_fd(), LOCK.as_ptr(), 0o600) },
        0
    );
    assert!(Owner::bind(root.path(), 1).is_err());
}

#[test]
fn t874_socket_hardlinks_are_not_eligible_for_stale_cleanup() {
    let root = root();
    let endpoint = root.path().join("broker.sock");
    drop(Listener::bind(&endpoint, 1).unwrap());
    std::fs::hard_link(&endpoint, root.path().join("alias")).unwrap();
    assert!(Owner::bind(root.path(), 1).is_err());
    assert!(endpoint.exists());
}
