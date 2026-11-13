//! Process exit observes original admissions; pools remain resource owners.
//! At most 12421 pending proofs are funded: session/route/input maxima of 4096 each,
//! plus 4 activity sources and the shared/config control-pool maxima of 64 each.
//! One additional receipt funds direct daemon keyboard restoration.
//! Custom control pools consume this same observation bound.
//! This separate bound includes the gap between pool release and publication.
//! Cutoff serializes with admission, wakes live supervisors, and keeps their
//! runtime alive until original native/task retirement publishes completion.
use blent_config::retirement::{Admission, Completion, Pool};
use std::sync::{LazyLock, Mutex};
use tokio_util::sync::CancellationToken;

pub(crate) static PROCESS: LazyLock<Registry> = LazyLock::new(Registry::default);

#[derive(Default)]
struct State {
    closed: bool,
    pending: Vec<Completion>,
}
#[derive(Default)]
pub(crate) struct Registry {
    state: Mutex<State>,
    stopping: CancellationToken,
}
impl Registry {
    pub(crate) fn reserve(&self, pool: &Pool) -> std::io::Result<Admission> {
        let mut state = self.state.lock().unwrap();
        Self::require_open(state.closed)?;
        state.pending.retain(|proof| !proof.is_complete());
        if state.pending.len() >= 12421 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Process retirement observation capacity exhausted",
            ));
        }
        let admission = pool.reserve()?;
        state.pending.push(admission.completion());
        Ok(admission)
    }
    pub(crate) async fn cancelled(&self) {
        self.stopping.cancelled().await;
    }
    fn require_open(closed: bool) -> std::io::Result<()> {
        if closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                crate::runtime_owner::AdmissionClosed,
            ));
        }
        Ok(())
    }
    pub(crate) fn check_running(&self) -> std::io::Result<()> {
        Self::require_open(self.state.lock().unwrap().closed)
    }
    pub(crate) fn close(&self) {
        self.state.lock().unwrap().closed = true;
        self.stopping.cancel();
    }
    fn settled(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.pending.retain(|proof| !proof.is_complete());
        state.pending.is_empty()
    }
    pub(crate) async fn drain(&self) {
        self.close();
        while !self.settled() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn t1411_cutoff_preserves_original_admissions_and_rejects_reuse() {
        let registry = Registry::default();
        let pool = Pool::new(1).unwrap();
        let original = registry.reserve(&pool).unwrap();
        assert!(registry.reserve(&pool).is_err());
        let draining = registry.drain();
        tokio::pin!(draining);
        assert!(futures_util::poll!(&mut draining).is_pending());
        registry.cancelled().await;
        assert!(registry.reserve(&pool).is_err());
        assert!(!original.completion().is_complete());
        drop(original);
        draining.await;
        assert!(registry.reserve(&pool).is_err());
    }

    #[tokio::test]
    async fn t1411_completed_history_is_bounded_and_empty_exit_finishes() {
        let registry = Registry::default();
        let pool = Pool::new(1).unwrap();
        for _ in 0..32 {
            drop(registry.reserve(&pool).unwrap());
            assert_eq!(registry.state.lock().unwrap().pending.len(), 1);
        }
        registry.drain().await;
        registry.drain().await;
        assert!(registry.settled());
    }
    #[tokio::test]
    async fn t1411_observation_capacity_is_independent_of_pool_release() {
        let registry = Registry::default();
        // T1421/T1422 activity/input, T1432 control, and T1441 direct restore maxima.
        let pools = [
            Pool::new(4096).unwrap(),
            Pool::new(4096).unwrap(),
            Pool::new(4).unwrap(),
            Pool::new(4096).unwrap(),
            Pool::new(64).unwrap(),
            Pool::new(64).unwrap(),
            Pool::new(1).unwrap(),
        ];
        let mut admitted = Vec::new();
        for pool in &pools {
            for _ in 0..pool.capacity() {
                admitted.push(registry.reserve(pool).unwrap());
            }
        }
        let spare = Pool::new(1).unwrap();
        assert_eq!(
            registry.reserve(&spare).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(spare.active(), 0);
        drop(admitted.pop());
        let replacement = registry.reserve(&spare).unwrap();
        assert_eq!(registry.state.lock().unwrap().pending.len(), 12421);
        drop((admitted, replacement));
        registry.drain().await;
    }
}
