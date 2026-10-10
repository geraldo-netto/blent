//! T824: contracts compile and behave without storage/platform features.
use blent_config::{model::FileConfig, persistence_contract::*};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Default)]
struct Memory(Mutex<FileConfig>);
impl Snapshots for Memory {
    fn snapshot(&self) -> FileConfig {
        self.0.lock().unwrap().clone()
    }
}
impl Transactions for Memory {
    fn transact(&self, cancelled: &AtomicBool, edit: Edit<'_>) -> anyhow::Result<FileConfig> {
        anyhow::ensure!(!cancelled.load(Ordering::Acquire), "cancelled");
        let mut stored = self.0.lock().unwrap();
        let mut next = stored.clone();
        edit(&mut next)?;
        next.sanitize_requested();
        next.validate()?;
        *stored = next.clone();
        Ok(next)
    }
}
impl SettingsEdits for Memory {
    fn merge_and_publish(
        &self,
        edited: &FileConfig,
        baseline: &FileConfig,
        publish: Publication<'_>,
    ) -> anyhow::Result<FileConfig> {
        let mut stored = self.0.lock().unwrap();
        let next = edited.merge_edits(baseline, stored.clone())?;
        *stored = next.clone();
        publish(&next);
        Ok(next)
    }
}

fn exercise(read: &dyn Snapshots, transactions: &dyn Transactions, edits: &dyn SettingsEdits) {
    let initial = read.snapshot();
    let cancelled = AtomicBool::new(true);
    assert!(transactions
        .transact(&cancelled, Box::new(|_| panic!("cancelled mutation ran")))
        .is_err());
    assert_eq!(read.snapshot(), initial);
    cancelled.store(false, Ordering::Release);
    for (invalid, bounded) in [(0, 10), (1, 10), (9, 10), (91, 90), (u32::MAX, 90)] {
        let config = transactions
            .transact(
                &cancelled,
                Box::new(|config| {
                    config.fps = invalid;
                    Ok(())
                }),
            )
            .unwrap();
        assert_eq!(config.fps, bounded);
        assert!(config.validate().is_ok());
        assert_eq!(read.snapshot(), config);
    }
    transactions
        .transact(
            &cancelled,
            Box::new(|config| {
                *config = initial.clone();
                Ok(())
            }),
        )
        .unwrap();
    assert!(transactions
        .transact(
            &cancelled,
            Box::new(|config| {
                config.pen_only = true;
                config.input_pen = false;
                Ok(())
            })
        )
        .is_err());
    assert_eq!(read.snapshot(), initial);
    let committed = transactions
        .transact(
            &cancelled,
            Box::new(|config| {
                config.fps = 30;
                Ok(())
            }),
        )
        .unwrap();
    assert_eq!(committed.fps, 30);
    let mut edited = initial.clone();
    edited.bitrate = 12_000;
    let mut published = None;
    let merged = edits
        .merge_and_publish(
            &edited,
            &initial,
            Box::new(|config| published = Some(config.clone())),
        )
        .unwrap();
    assert_eq!(merged.fps, 30);
    assert_eq!(merged.bitrate, 12_000);
    assert_eq!(published, Some(merged.clone()));
    assert_eq!(read.snapshot(), merged);
}
#[test]
fn t824_memory_adapter_preserves_cancellation_atomic_rejection_and_merge_contract() {
    let store = Memory::default();
    exercise(&store, &store, &store);
}
#[cfg(feature = "storage")]
#[test]
fn t824_filesystem_adapter_obeys_the_same_contract() {
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    exercise(&store, &store, &store);
}
