//! T566: successful-write diagnostics must follow the filesystem commit.
use super::*;

#[test]
fn t1531_success_requires_directory_sync_after_complete_publication() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let config = FileConfig {
        fps: 30,
        ..Default::default()
    };
    let mut synced = false;
    write_with_sync(&config, &path, |parent| {
        assert_eq!(parent, directory.path());
        let published: FileConfig = toml::from_str(&std::fs::read_to_string(&path)?).unwrap();
        assert_eq!(published, config, "T1531: sync preceded publication");
        synced = true;
        Ok(())
    })
    .unwrap();
    assert!(synced, "T1531: successful save omitted directory sync");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn t1531_sync_failure_is_reported_without_undoing_visible_publication() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let config = FileConfig {
        fps: 30,
        ..Default::default()
    };
    let result = write_with_sync(&config, &path, |_| anyhow::bail!("injected sync failure"));
    assert!(result.is_err(), "T1531: failed durability claimed success");
    assert_eq!(load_at(&path), config);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn t1531_rejected_publication_does_not_sync_directory() {
    let directory = tempfile::tempdir().unwrap();
    let config = FileConfig::default();
    let result = write_with_sync(&config, directory.path(), |_| {
        panic!("T1531: sync after failed rename")
    });
    assert!(result.is_err());
    assert!(directory.path().is_dir());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn t1531_native_directory_sync_reports_invalid_directory() {
    let directory = tempfile::tempdir().unwrap();
    sync_directory(directory.path()).unwrap();
    assert!(sync_directory(&directory.path().join("absent")).is_err());
}

fn recorded_write(config: &FileConfig, path: &Path) -> (Result<()>, String) {
    let log = tempfile::NamedTempFile::new().unwrap();
    let writer = log.reopen().unwrap();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.try_clone().unwrap())
        .finish();
    let result = tracing::subscriber::with_default(subscriber, || write_at(config, path));
    (result, std::fs::read_to_string(log.path()).unwrap())
}

#[cfg(target_os = "linux")]
#[test]
fn t566_failed_persistence_never_reports_a_successful_change() {
    use std::os::fd::AsRawFd;
    let original = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(original.path(), "fps = 60\n").unwrap();
    // procfs permits reading the old regular file but forbids creating the
    // temporary replacement, including under root in the container suite.
    let path = PathBuf::from(format!("/proc/self/fd/{}", original.as_file().as_raw_fd()));
    let changed = FileConfig {
        fps: 30,
        ..Default::default()
    };
    let (result, log) = recorded_write(&changed, &path);
    assert!(result.is_err(), "T566: fixture must fail at persistence");
    assert_eq!(
        std::fs::read_to_string(original.path()).unwrap(),
        "fps = 60\n"
    );
    assert!(
        !log.contains("Config written:"),
        "T566: failed commit claimed success: {log}"
    );
}

#[test]
fn t566_success_reports_only_committed_changes_and_noop_stays_quiet() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let original = FileConfig::default();
    write_at(&original, &path).unwrap();
    let changed = FileConfig {
        fps: 30,
        ..original
    };
    let (result, log) = recorded_write(&changed, &path);
    result.unwrap();
    assert_eq!(crate::storage::load_at(&path).fps, 30);
    assert!(
        log.contains("Config written: fps = 30 (was 60)"),
        "T566: {log}"
    );
    let (result, log) = recorded_write(&changed, &path);
    result.unwrap();
    assert!(
        !log.contains("Config written:"),
        "T566: unchanged configuration: {log}"
    );
}
