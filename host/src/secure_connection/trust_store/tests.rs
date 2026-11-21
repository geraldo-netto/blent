use super::*;
use blent_config::connection::{
    pairing::{Pairing, PeerId},
    trust::{codec, Snapshot, Trust, DEFAULT_CAPACITY},
};
use std::os::unix::fs::{symlink, PermissionsExt};

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn peer(value: char) -> PeerId {
    PeerId::parse(value.to_string().repeat(64)).unwrap()
}
fn record(present: bool) -> Vec<u8> {
    codec::encode(&Snapshot {
        local: peer('0'),
        revision: if present { 1 } else { 2 },
        peers: if present { vec![peer('1')] } else { vec![] },
    })
    .unwrap()
}
#[test]
fn t868_native_restart_remembers_and_revokes_with_exclusive_ownership() {
    let root = root();
    let store = LinuxTrustStore::open(root.path()).unwrap();
    assert!(LinuxTrustStore::open(root.path()).is_err());
    let mut trust = Trust::open(store, peer('0'), DEFAULT_CAPACITY).unwrap();
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(peer('1'), false, 0, 100).unwrap();
    pairing.approve(&ticket, true, 1).unwrap();
    let lease = pairing.authenticated(&ticket, &peer('1'), 2).unwrap();
    trust.remember(&pairing, &lease).unwrap();
    drop(trust);
    let mut trust = Trust::open(
        LinuxTrustStore::open(root.path()).unwrap(),
        peer('0'),
        DEFAULT_CAPACITY,
    )
    .unwrap();
    assert!(trust.trusted(&peer('1')));
    trust.revoke(&mut pairing, &peer('1')).unwrap();
    assert!(!pairing.current(&lease));
    drop(trust);
    let trust = Trust::open(
        LinuxTrustStore::open(root.path()).unwrap(),
        peer('0'),
        DEFAULT_CAPACITY,
    )
    .unwrap();
    assert!(!trust.trusted(&peer('1')));
    assert!(!root.path().join("trust.pending").exists());
    assert!(!root.path().join("trust.next").exists());
}
thread_local! { static FAULT: std::cell::Cell<Option<(&'static str, bool)>> = const { std::cell::Cell::new(None) }; }
const POINTS: [&str; 8] = [
    "marker-sync",
    "intent-sync",
    "write",
    "record-sync",
    "rename",
    "publication-sync",
    "remove-marker",
    "retirement-sync",
];
pub(super) fn checkpoint(point: &str) -> Result<()> {
    if let Some((selected, crash)) = FAULT.get() {
        if selected == point {
            if crash {
                std::process::exit(86);
            }
            anyhow::bail!("injected trust failure at {point}");
        }
    }
    Ok(())
}
#[test]
fn t868_failed_revoke_at_each_boundary_never_automatically_restores_old_trust() {
    for point in POINTS {
        let root = root();
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        store.replace(&record(true)).unwrap();
        FAULT.set(Some((point, false)));
        let result = store.replace(&record(false));
        FAULT.set(None);
        assert!(result.is_err(), "T868 {point}");
        drop(store);
        rejected_or_revoked(root.path());
    }
}
fn rejected_or_revoked(path: &Path) {
    let store = LinuxTrustStore::open(path).unwrap();
    let result = Trust::open(store, peer('0'), DEFAULT_CAPACITY);
    if let Ok(trust) = result {
        assert!(!trust.trusted(&peer('1')));
    }
}
#[test]
fn t868_crashed_revoke_at_each_boundary_never_automatically_restores_old_trust() {
    for point in POINTS {
        let root = root();
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        store.replace(&record(true)).unwrap();
        drop(store);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "secure_connection::trust_store::tests::t868_crash_child",
                "--nocapture",
            ])
            .env("BLENT_T868_CRASH_POINT", point)
            .env("BLENT_T868_CRASH_DIR", root.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let status = bounded_wait(&mut child);
        assert_eq!(status.code(), Some(86), "T868 {point}");
        rejected_or_revoked(root.path());
    }
}
fn bounded_wait(child: &mut std::process::Child) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("T868 crash fixture deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[test]
fn t868_crash_child() {
    let Ok(point) = std::env::var("BLENT_T868_CRASH_POINT") else {
        return;
    };
    let selected = POINTS
        .into_iter()
        .find(|candidate| *candidate == point)
        .unwrap();
    let root = std::env::var_os("BLENT_T868_CRASH_DIR").unwrap();
    let mut store = LinuxTrustStore::open(Path::new(&root)).unwrap();
    FAULT.set(Some((selected, true)));
    let _ = store.replace(&record(false));
    panic!("T868 missed crash checkpoint");
}
#[test]
fn t868_native_incomplete_malformed_and_oversized_state_is_preserved() {
    for name in ["trust.pending", "trust.next", "trust.bin"] {
        let root = root();
        std::fs::write(root.path().join(name), b"incomplete").unwrap();
        std::fs::set_permissions(
            root.path().join(name),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert!(Trust::open(LinuxTrustStore::open(root.path()).unwrap(), peer('0'), 64).is_err());
        assert_eq!(
            std::fs::read(root.path().join(name)).unwrap(),
            b"incomplete"
        );
    }
    let root = root();
    let mut store = LinuxTrustStore::open(root.path()).unwrap();
    assert!(store.replace(&vec![0; MAX_RECORD + 1]).is_err());
    assert!(store.load().unwrap().is_none());
    assert!(store.directory.remove(c"absent").is_err());
}
#[test]
fn t868_native_symlink_and_failed_rename_preserve_target_and_intent() {
    let root = root();
    let target = root.path().join("target");
    std::fs::write(&target, b"untouched").unwrap();
    symlink(&target, root.path().join("trust.bin")).unwrap();
    let mut store = LinuxTrustStore::open(root.path()).unwrap();
    assert!(store.load().is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"untouched");
    std::fs::remove_file(root.path().join("trust.bin")).unwrap();
    std::fs::create_dir(root.path().join("trust.bin")).unwrap();
    assert!(store.replace(&record(true)).is_err());
    assert!(store.load().is_err());
    assert!(root.path().join("trust.pending").is_file());
    assert!(root.path().join("trust.next").is_file());
}

#[test]
fn t1635_restart_completes_every_admitted_revocation() {
    for point in POINTS {
        let root = root();
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        store.replace(&record(true)).unwrap();
        FAULT.set(Some((point, false)));
        let result = store.replace(&record(false));
        FAULT.set(None);
        assert!(result.is_err(), "T1635 {point}");
        drop(store);
        let mut reopened = LinuxTrustStore::open(root.path()).unwrap();
        assert_eq!(
            reopened.load().unwrap(),
            Some(record(false)),
            "T1635 {point}"
        );
        reopened.replace(&record(true)).unwrap();
        assert_eq!(reopened.load().unwrap(), Some(record(true)));
    }
}

const RESET_POINTS: [&str; 6] = [
    "reset-intent-sync",
    "trust.bin",
    "trust.next",
    "trust.pending",
    "reset-remove",
    "reset-retirement-sync",
];

fn prepare_reset(root: &Path) {
    let directory = Directory::open(root, c"trust.lock").unwrap();
    for name in [
        c"trust.bin",
        c"trust.next",
        c"trust.pending",
        c"identity.bin",
    ] {
        directory
            .child(name, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)
            .unwrap()
            .write_all(b"retain unless explicitly reset")
            .unwrap();
    }
}

fn reset_outcome(root: &Path) {
    let mut store = LinuxTrustStore::open(root).unwrap();
    assert!(store.load().unwrap().is_none());
    assert_eq!(
        std::fs::read(root.join("identity.bin")).unwrap(),
        b"retain unless explicitly reset"
    );
    assert!(root.join("trust.lock").is_file());
    assert!(!root.join("trust.reset").exists());
    store.reset().unwrap();
    store.replace(&record(false)).unwrap();
    assert_eq!(store.load().unwrap(), Some(record(false)));
}

#[test]
fn t1635_reset_resumes_every_failed_transition_and_preserves_identity() {
    for point in RESET_POINTS {
        let root = root();
        prepare_reset(root.path());
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        FAULT.set(Some((point, false)));
        let result = store.reset();
        FAULT.set(None);
        assert!(result.is_err(), "T1635 reset {point}");
        assert!(LinuxTrustStore::open(root.path()).is_err());
        drop(store);
        reset_outcome(root.path());
    }
}

#[test]
fn t1635_reset_crash_child() {
    let Ok(point) = std::env::var("BLENT_T1635_RESET_POINT") else {
        return;
    };
    let selected = RESET_POINTS
        .into_iter()
        .find(|candidate| *candidate == point)
        .unwrap();
    let root = std::env::var_os("BLENT_T1635_RESET_DIR").unwrap();
    let mut store = LinuxTrustStore::open(Path::new(&root)).unwrap();
    FAULT.set(Some((selected, true)));
    let _ = store.reset();
    panic!("T1635 reset missed crash checkpoint");
}

#[test]
fn t1635_reset_survives_original_process_exit_at_each_transition() {
    for point in RESET_POINTS {
        let root = root();
        prepare_reset(root.path());
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "secure_connection::trust_store::tests::t1635_reset_crash_child",
                "--nocapture",
            ])
            .env("BLENT_T1635_RESET_POINT", point)
            .env("BLENT_T1635_RESET_DIR", root.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        assert_eq!(bounded_wait(&mut child).code(), Some(86));
        reset_outcome(root.path());
    }
}

#[test]
fn t1635_truncated_intents_require_explicit_reset() {
    let intended = record(false);
    for size in 0..intended.len() {
        let root = root();
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        store.replace(&record(true)).unwrap();
        store
            .directory
            .child(
                c"trust.pending",
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )
            .unwrap()
            .write_all(&intended[..size])
            .unwrap();
        assert!(store.load().is_err());
        assert_eq!(
            std::fs::read(root.path().join("trust.bin")).unwrap(),
            record(true)
        );
        assert_eq!(
            std::fs::read(root.path().join("trust.pending")).unwrap(),
            intended[..size]
        );
        store.reset().unwrap();
        assert!(store.load().unwrap().is_none());
    }
}

#[test]
fn t1635_recovery_can_itself_be_interrupted_without_restoring_approval() {
    for point in &POINTS[2..] {
        let root = root();
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        store.replace(&record(true)).unwrap();
        store
            .directory
            .child(
                c"trust.pending",
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            )
            .unwrap()
            .write_all(&record(false))
            .unwrap();
        FAULT.set(Some((*point, false)));
        let result = store.load();
        FAULT.set(None);
        assert!(result.is_err(), "T1635 recovery {point}");
        drop(store);
        assert_eq!(
            LinuxTrustStore::open(root.path()).unwrap().load().unwrap(),
            Some(record(false))
        );
    }
}

#[test]
fn t1635_reset_rejects_unsafe_children_without_touching_targets() {
    for kind in ["symlink", "hardlink", "mode", "directory", "fifo"] {
        let root = root();
        let target = root.path().join("external");
        std::fs::write(&target, b"keep").unwrap();
        let path = root.path().join("trust.bin");
        unsafe_reset_fixture(kind, &target, &path);
        let mut store = LinuxTrustStore::open(root.path()).unwrap();
        assert!(store.reset().is_err());
        assert!(store.load().is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"keep");
        assert!(root.path().join("trust.reset").is_file());
    }
}

fn unsafe_reset_fixture(kind: &str, target: &Path, path: &Path) {
    match kind {
        "symlink" => symlink(target, path).unwrap(),
        "hardlink" => std::fs::hard_link(target, path).unwrap(),
        "mode" => {
            std::fs::write(path, b"keep").unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        "fifo" => {
            use std::os::unix::ffi::OsStrExt;
            let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        }
        _ => std::fs::create_dir(path).unwrap(),
    }
}
