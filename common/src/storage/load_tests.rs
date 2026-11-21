//! T1532: best-effort startup must disclose unreadable settings.
use super::*;

fn recorded_load(path: &Path) -> (FileConfig, String) {
    let log = tempfile::NamedTempFile::new().unwrap();
    let writer = log.reopen().unwrap();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.try_clone().unwrap())
        .finish();
    let config = tracing::subscriber::with_default(subscriber, || load_at(path));
    (config, std::fs::read_to_string(log.path()).unwrap())
}

#[test]
fn t1532_unreadable_config_reports_path_and_fallback_without_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    for unreadable in [&path, directory.path()] {
        let (config, log) = recorded_load(unreadable);
        assert_eq!(config, FileConfig::default());
        assert!(log.contains("Cannot read config"), "T1532: {log}");
        assert!(log.contains(unreadable.to_str().unwrap()), "T1532: {log}");
        assert!(log.contains("using defaults"), "T1532: {log}");
    }
    assert_eq!(std::fs::read(&path).unwrap(), [0xff, 0xfe]);
}

#[test]
fn t1532_only_missing_or_valid_config_is_quiet() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let (_, log) = recorded_load(&path);
    assert!(log.is_empty(), "T1532: missing config: {log}");
    std::fs::write(&path, "fps = 30\n").unwrap();
    let (config, log) = recorded_load(&path);
    assert_eq!(config.fps, 30);
    assert!(log.is_empty(), "T1532: valid config: {log}");
    std::fs::write(&path, "fps = [not a number").unwrap();
    let (_, log) = recorded_load(&path);
    assert!(
        log.contains("Invalid config"),
        "T1532: malformed config: {log}"
    );
}
