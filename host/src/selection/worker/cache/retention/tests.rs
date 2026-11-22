use super::*;
use std::{
    os::unix::{ffi::OsStrExt, fs::symlink},
    time::Duration,
};

fn write_record(root: &Path, index: u32) -> PathBuf {
    let path = root.join(format!("{index:064x}.json"));
    fs::write(&path, b"record").unwrap();
    fs::File::open(&path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(u64::from(index)))
        .unwrap();
    path
}

#[test]
fn t1490_retention_has_configurable_bounds_and_keeps_current_even_when_oldest() {
    for capacity in [1, 2, 64, 256] {
        let root = tempfile::tempdir().unwrap();
        let current = write_record(root.path(), 0);
        for index in 1..capacity + 5 {
            write_record(root.path(), index);
        }
        let transaction = Transaction::open(root.path()).unwrap();
        transaction
            .reserve(current.file_name().unwrap(), capacity)
            .unwrap();
        assert!(current.is_file());
        for index in 1..capacity + 5 {
            let expected = index >= 6;
            assert_eq!(
                root.path().join(format!("{index:064x}.json")).exists(),
                expected,
                "capacity {capacity}, index {index}"
            );
        }
        assert!(!transaction
            .pass(current.file_name().unwrap(), capacity as usize)
            .unwrap());
    }
}

#[test]
fn t1490_retention_preserves_foreign_names_links_and_directories() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "external").unwrap();
    let link = root.path().join(format!("{:064x}.json", 100));
    symlink(outside.path(), &link).unwrap();
    let directory = root.path().join(format!("{:064x}.json", 101));
    fs::create_dir(&directory).unwrap();
    for name in ["notes.json", "temporary", "UPPER.json"] {
        fs::write(root.path().join(name), "keep").unwrap();
    }
    let current = write_record(root.path(), 0);
    let old = write_record(root.path(), 1);
    Transaction::open(root.path())
        .unwrap()
        .reserve(current.file_name().unwrap(), 1)
        .unwrap();
    assert!(!old.exists());
    assert!(link.is_symlink());
    assert!(directory.is_dir());
    assert_eq!(fs::read_to_string(outside.path()).unwrap(), "external");
    for name in ["notes.json", "temporary", "UPPER.json"] {
        assert_eq!(fs::read_to_string(root.path().join(name)).unwrap(), "keep");
    }
}

#[test]
fn t1490_retention_names_have_exact_bounds_and_lowercase_hex() {
    for stem in ["0".repeat(64), "a".repeat(64), "f".repeat(64)] {
        assert!(record_name(OsStr::new(&format!("{stem}.json"))));
    }
    for name in [
        ".json".into(),
        format!("{}.json", "a".repeat(63)),
        format!("{}.json", "a".repeat(65)),
        format!("{}.json", "A".repeat(64)),
        format!("{}.json", "g".repeat(64)),
        format!("{}.JSON", "a".repeat(64)),
    ] {
        assert!(!record_name(OsStr::new(&name)));
    }
    assert!(!record_name(OsStr::from_bytes(&[255])));
}

#[test]
fn t1490_bad_lock_paths_and_capacity_fail_before_retention() {
    let root = tempfile::tempdir().unwrap();
    for capacity in [0, 257, u32::MAX] {
        let transaction = Transaction::open(root.path()).unwrap();
        assert!(transaction
            .reserve(OsStr::new("current"), capacity)
            .is_err());
    }
    fs::remove_file(root.path().join("retention.lock")).unwrap();
    assert!(Transaction::open(&root.path().join("missing")).is_err());
    let foreign = tempfile::NamedTempFile::new().unwrap();
    let lock = root.path().join("retention.lock");
    symlink(foreign.path(), &lock).unwrap();
    assert!(Transaction::open(root.path()).is_err());
    fs::remove_file(&lock).unwrap();
    assert!(std::process::Command::new("mkfifo")
        .arg(&lock)
        .status()
        .unwrap()
        .success());
    assert!(Transaction::open(root.path()).is_err());
}

#[test]
fn t1490_lock_excludes_writers_and_releases_on_drop() {
    let root = tempfile::tempdir().unwrap();
    let first = Transaction::open(root.path()).unwrap();
    assert!(Transaction::open(root.path()).is_err());
    drop(first);
    assert!(Transaction::open(root.path()).is_ok());
}

#[test]
fn t1490_missing_names_settle_and_nonfile_removal_errors_are_reported() {
    let root = tempfile::tempdir().unwrap();
    assert!(remove(&root.path().join("absent")).is_ok());
    assert!(remove(root.path()).is_err());
}

fn check_process_lock(root: &Path, busy: bool) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "selection::worker::cache::retention::tests::t1490_lock_exclusion_applies_across_original_processes", "--nocapture"])
        .env("BLENT_T1490_LOCK_PATH", root).env("BLENT_T1490_LOCK_BUSY", busy.to_string()).output().unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn t1490_lock_exclusion_applies_across_original_processes() {
    if let Some(path) = std::env::var_os("BLENT_T1490_LOCK_PATH") {
        let busy = std::env::var("BLENT_T1490_LOCK_BUSY").unwrap() == "true";
        assert_eq!(Transaction::open(Path::new(&path)).is_err(), busy);
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let transaction = Transaction::open(root.path()).unwrap();
    check_process_lock(root.path(), true);
    drop(transaction);
    check_process_lock(root.path(), false);
}

#[test]
fn t1490_retention_stays_with_original_directory_after_path_replacement() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("cache");
    fs::create_dir(&original).unwrap();
    let current = write_record(&original, 0);
    write_record(&original, 1);
    let transaction = Transaction::open(&original).unwrap();
    let moved = root.path().join("original");
    fs::rename(&original, &moved).unwrap();
    fs::create_dir(&original).unwrap();
    let foreign = write_record(&original, 2);
    transaction
        .reserve(current.file_name().unwrap(), 1)
        .unwrap();
    assert!(
        foreign.is_file(),
        "T1490 retention followed a replaced directory path"
    );
    assert!(!moved.join(format!("{:064x}.json", 1)).exists());
    transaction
        .publish(current.file_name().unwrap(), b"published", 1)
        .unwrap();
    assert_eq!(
        fs::read(moved.join(current.file_name().unwrap())).unwrap(),
        b"published"
    );
    assert!(foreign.is_file());
}

#[test]
fn t1490_failed_publication_cleans_tempfiles_before_releasing_directory() {
    let root = tempfile::tempdir().unwrap();
    let transaction = Transaction::open(root.path()).unwrap();
    fs::create_dir(root.path().join("target.json")).unwrap();
    assert!(transaction
        .publish(OsStr::new("target.json"), b"record", 64)
        .is_err());
    for name in ["", ".", "..", "../outside", "/absolute", "nested/file"] {
        assert!(transaction
            .publish(OsStr::new(name), b"record", 64)
            .is_err());
    }
    assert!(transaction
        .publish(
            OsStr::new("current.json"),
            &vec![0; super::super::MAX_BYTES as usize + 1],
            64
        )
        .is_err());
    assert!(transaction
        .publish(OsStr::new("current.json"), b"record", 0)
        .is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    drop(transaction);
    assert!(Transaction::open(root.path()).is_ok());
}
