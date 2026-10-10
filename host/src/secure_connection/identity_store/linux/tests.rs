use super::*;
use crate::secure_connection::{identity_store::load_or_create, IdentityRecord};
use std::os::unix::fs::{symlink, PermissionsExt};

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn private_file(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
#[test]
fn t867_native_restart_reuses_identity_and_concurrent_owner_is_rejected() {
    let root = root();
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    assert!(LinuxIdentityStore::open(root.path()).is_err());
    let identity = load_or_create(&mut store, IdentityRecord::generate).unwrap();
    assert!(store.reserve().is_err());
    assert!(store.publish(b"not reserved").is_err());
    drop(store);
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    let reused = load_or_create(&mut store, || panic!("T867 restart rotated identity")).unwrap();
    assert_eq!(identity.certificate(), reused.certificate());
    assert_eq!(
        std::fs::metadata(root.path().join("identity.bin"))
            .unwrap()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!root.path().join("identity.pending").exists());
}
#[test]
fn t867_pending_initialization_is_durable_and_never_silently_replaced() {
    let root = root();
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    store.reserve().unwrap();
    assert!(store.reserve().is_err());
    assert!(store.load().is_err());
    drop(store);
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    assert!(load_or_create(&mut store, || panic!(
        "T867 interrupted initialization rotated"
    ))
    .is_err());
    assert_eq!(
        std::fs::read(root.path().join("identity.pending")).unwrap(),
        b""
    );
    assert!(store.publish(b"not reserved after restart").is_err());
}
#[test]
fn t867_failed_publication_preserves_conflicting_and_pending_records() {
    let root = root();
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    store.reserve().unwrap();
    private_file(&root.path().join("identity.bin"), b"external state");
    assert!(store.publish(b"pending state").is_err());
    assert!(store.load().is_err());
    assert_eq!(
        std::fs::read(root.path().join("identity.bin")).unwrap(),
        b"external state"
    );
    assert_eq!(
        std::fs::read(root.path().join("identity.pending")).unwrap(),
        b"pending state"
    );
    assert!(store.publish(b"retry").is_err());
}
#[test]
fn t867_oversized_and_corrupt_records_are_bounded_and_preserved() {
    for size in [0, 16, MAX_RECORD, MAX_RECORD + 1, MAX_RECORD * 2] {
        let root = root();
        let bytes = vec![0; size];
        private_file(&root.path().join("identity.bin"), &bytes);
        let mut store = LinuxIdentityStore::open(root.path()).unwrap();
        assert!(load_or_create(&mut store, || panic!("T867 malformed identity rotated")).is_err());
        assert_eq!(
            std::fs::read(root.path().join("identity.bin")).unwrap(),
            bytes
        );
    }
    let root = root();
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    store.reserve().unwrap();
    assert!(store.publish(&vec![0; MAX_RECORD + 1]).is_err());
    assert_eq!(
        std::fs::read(root.path().join("identity.pending")).unwrap(),
        b""
    );
}
#[test]
fn t867_private_directory_is_required_and_pinned_across_rename() {
    let root = root();
    let path = root.path().join("identity");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(LinuxIdentityStore::open(&path).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut store = LinuxIdentityStore::open(&path).unwrap();
    let moved = root.path().join("moved");
    std::fs::rename(&path, &moved).unwrap();
    std::fs::create_dir(&path).unwrap();
    load_or_create(&mut store, IdentityRecord::generate).unwrap();
    assert!(moved.join("identity.bin").exists());
    assert!(!path.join("identity.bin").exists());
    let link = root.path().join("link");
    symlink(&moved, &link).unwrap();
    assert!(LinuxIdentityStore::open(&link).is_err());
}
#[test]
fn t867_unsafe_children_are_rejected_without_touching_targets() {
    for name in ["identity.lock", "identity.bin", "identity.pending"] {
        for kind in ["link", "hardlink", "mode", "directory", "fifo"] {
            unsafe_child(name, kind);
        }
    }
}
fn unsafe_child(name: &str, kind: &str) {
    let root = root();
    let target = root.path().join("target");
    private_file(&target, b"keep");
    let child = root.path().join(name);
    match kind {
        "link" => symlink(&target, &child).unwrap(),
        "hardlink" => std::fs::hard_link(&target, &child).unwrap(),
        "mode" => {
            private_file(&child, b"keep");
            std::fs::set_permissions(&child, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        "fifo" => {
            use std::os::unix::ffi::OsStrExt;
            let path = std::ffi::CString::new(child.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        }
        _ => std::fs::create_dir(&child).unwrap(),
    }
    let result = LinuxIdentityStore::open(root.path()).and_then(|mut store| {
        load_or_create(&mut store, || panic!("T867 unsafe store rotated identity"))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"keep");
}

// Faults are test-only and thread-local; no environment-selected production I/O.
thread_local! { static FAULT: std::cell::Cell<Option<(&'static str, bool)>> = const { std::cell::Cell::new(None) }; }
const POINTS: [&str; 8] = [
    "pending-sync",
    "reserve-sync",
    "reserved",
    "write",
    "record-sync",
    "rename",
    "publish-sync",
    "published",
];
pub(super) fn checkpoint(point: &str) -> Result<()> {
    if let Some((selected, crash)) = FAULT.get() {
        if selected == point {
            if crash {
                std::process::exit(86);
            }
            anyhow::bail!("injected persistence failure at {point}");
        }
    }
    Ok(())
}
#[test]
fn t867_every_failed_storage_boundary_prevents_regeneration() {
    for point in POINTS {
        let root = root();
        let mut store = LinuxIdentityStore::open(root.path()).unwrap();
        FAULT.set(Some((point, false)));
        let result = load_or_create(&mut store, IdentityRecord::generate);
        FAULT.set(None);
        assert!(result.is_err(), "T867 {point}");
        drop(store);
        assert_no_regeneration(root.path());
    }
}
fn assert_no_regeneration(path: &Path) {
    let mut store = LinuxIdentityStore::open(path).unwrap();
    let result = load_or_create(&mut store, || panic!("T867 rotated interrupted identity"));
    assert_eq!(result.is_ok(), path.join("identity.bin").exists());
}
#[test]
fn t867_process_exit_at_each_boundary_preserves_identity_or_pending_state() {
    for point in POINTS {
        let root = root();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "secure_connection::identity_store::linux::tests::t867_crash_child",
                "--nocapture",
            ])
            .env("BLENT_T867_CRASH_POINT", point)
            .env("BLENT_T867_CRASH_DIR", root.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(86), "T867 {point}");
        assert_no_regeneration(root.path());
    }
}
#[test]
fn t867_crash_child() {
    let Ok(point) = std::env::var("BLENT_T867_CRASH_POINT") else {
        return;
    };
    let selected = POINTS
        .into_iter()
        .find(|candidate| *candidate == point)
        .unwrap();
    let root = std::env::var_os("BLENT_T867_CRASH_DIR").unwrap();
    let mut store = LinuxIdentityStore::open(Path::new(&root)).unwrap();
    FAULT.set(Some((selected, true)));
    let _ = load_or_create(&mut store, IdentityRecord::generate);
    panic!("T867 missed crash checkpoint");
}
#[test]
fn t867_native_write_failure_keeps_reserved_record() {
    let root = root();
    let mut store = LinuxIdentityStore::open(root.path()).unwrap();
    store.reserve().unwrap();
    store.pending = Some(File::open(root.path().join("identity.pending")).unwrap());
    assert!(store.publish(b"record").is_err()); // Real EBADF from read-only fd.
    drop(store);
    assert_no_regeneration(root.path());
}
