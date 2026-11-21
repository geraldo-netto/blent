//! A polling read never replaces an unavailable or malformed file with defaults.
use super::ConfigStore;
use crate::model::FileConfig;
use crate::persistence_contract::ObservedSnapshots;
use anyhow::{ensure, Context, Result};
use std::io::Read;

const MAX_SNAPSHOT_BYTES: u64 = 1024 * 1024;

impl ObservedSnapshots for ConfigStore {
    fn observe(&self) -> Result<FileConfig> {
        let file = open_snapshot(self.path()?)?;
        let mut text = String::new();
        file.take(MAX_SNAPSHOT_BYTES + 1)
            .read_to_string(&mut text)?;
        ensure!(
            text.len() as u64 <= MAX_SNAPSHOT_BYTES,
            "configuration snapshot exceeds 1 MiB"
        );
        let mut config: FileConfig =
            toml::from_str(&text).context("parse observed configuration")?;
        config.sanitize();
        config.validate()?;
        Ok(config)
    }
}

fn open_snapshot(path: &std::path::Path) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "configuration snapshot must be a regular file"
    );
    ensure!(
        metadata.len() <= MAX_SNAPSHOT_BYTES,
        "configuration snapshot exceeds 1 MiB"
    );
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1552_snapshot_rejects_unavailable_malformed_and_oversized_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let store = ConfigStore::new(path.clone());
        assert!(store.observe().is_err());
        for text in [b"bad = [".as_slice(), &[0xff], b"width = 1"] {
            std::fs::write(&path, text).unwrap();
            // Numeric values follow the existing sanitization policy.
            let result = store.observe();
            if text == b"width = 1" {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        }
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_SNAPSHOT_BYTES + 1).unwrap();
        assert!(store.observe().is_err());
        drop(file);
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(store.observe().is_err());
    }

    #[test]
    fn t1552_observation_matches_saved_current_config() {
        let directory = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(directory.path().join("config.toml"));
        let config = FileConfig::default();
        store.save(&config).unwrap();
        assert_eq!(store.observe().unwrap(), config);
    }

    #[cfg(unix)]
    #[test]
    fn t1552_fifo_snapshot_does_not_wait_for_a_writer() {
        use std::os::unix::ffi::OsStrExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fifo");
        let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
        assert!(ConfigStore::new(path).observe().is_err());
    }
}

#[cfg(test)]
#[test]
fn t1552_observed_text_capacity_boundaries() {
    use crate::persistence_contract::ObservedSnapshots;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let store = ConfigStore::new(path.clone());
    for length in [
        0,
        1,
        MAX_SNAPSHOT_BYTES - 1,
        MAX_SNAPSHOT_BYTES,
        MAX_SNAPSHOT_BYTES + 1,
    ] {
        // A comment is valid TOML at every accepted length.
        std::fs::write(&path, vec![b'#'; length as usize]).unwrap();
        assert_eq!(store.observe().is_ok(), length <= MAX_SNAPSHOT_BYTES);
    }
}
