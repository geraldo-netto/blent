//! Portable persistence ports. These contracts contain no paths, locks, storage
//! implementation or native service types. Validation/merge stay on FileConfig.
use crate::model::FileConfig;
pub type PersistenceResult<T> = anyhow::Result<T>;
use std::sync::atomic::AtomicBool;

pub type Edit<'a> = Box<dyn FnOnce(&mut FileConfig) -> PersistenceResult<()> + Send + 'a>;
pub type Publication<'a> = Box<dyn FnOnce(&FileConfig) + Send + 'a>;

pub trait Snapshots: Send + Sync {
    /// Best-effort sanitized snapshot; unavailable/malformed storage uses the
    /// same domain defaults as a missing configuration. This does not authorize
    /// an update to overwrite an unreadable persisted value.
    fn snapshot(&self) -> FileConfig;
}

pub trait Transactions: Send + Sync {
    /// Serialize read/edit/validation/commit. Invoke edit at most once, only
    /// after reading successfully. Cancellation may reject before commit; once
    /// commit starts the returned result must describe its actual completion.
    /// Dropping a waiter does not roll back an already-started transaction.
    fn transact(&self, cancelled: &AtomicBool, edit: Edit<'_>) -> PersistenceResult<FileConfig>;
}

pub trait SettingsEdits: Send + Sync {
    /// Merge changed fields over the latest stored snapshot. Publication runs
    /// exactly once after a successful commit, under the same serialization
    /// boundary. Publication failure cannot undo that commit. The caller owns
    /// any restart and must perform it only after this method returns success.
    fn merge_and_publish(
        &self,
        edited: &FileConfig,
        baseline: &FileConfig,
        publish: Publication<'_>,
    ) -> PersistenceResult<FileConfig>;
}
