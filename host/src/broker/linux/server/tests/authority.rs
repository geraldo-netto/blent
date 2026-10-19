use super::*;
use crate::broker::linux::server::authority as admission;
use blent_config::connection::trust::TrustStorage;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc, Mutex,
};

#[tokio::test]
async fn t1100_default_start_rejects_process_authority_exhaustion_before_effects() {
    let _guard = admission::test_guard();
    let pool = admission::shared().unwrap();
    let held: Vec<_> = (0..pool.capacity())
        .map(|_| pool.reserve().unwrap())
        .collect();
    let root = tempfile::tempdir().unwrap();
    let settings = Settings::new(
        root.path().join("runtime"),
        root.path().join("state"),
        root.path().join("helper"),
    );
    let runtime = settings.runtime.clone();
    let state = settings.state.clone();
    let attempt = Server::open(settings).await;
    let bypassed = attempt.is_ok();
    if let Ok(mut unexpected) = attempt {
        unexpected.shutdown(Duration::from_secs(2)).await.unwrap();
        drop(unexpected);
    }
    drop(held);
    assert!(
        !bypassed,
        "fresh broker pool bypassed original process admission"
    );
    assert!(!runtime.exists());
    assert!(!state.exists());
}

struct Stage {
    entered: AtomicBool,
    release: Mutex<Receiver<()>>,
}
impl Stage {
    fn new() -> (Arc<Self>, SyncSender<()>) {
        let (send, receive) = sync_channel(1);
        (
            Arc::new(Self {
                entered: AtomicBool::new(false),
                release: Mutex::new(receive),
            }),
            send,
        )
    }
    fn wait(&self) {
        self.entered.store(true, Ordering::Release);
        self.release.lock().unwrap().recv().unwrap();
    }
}
struct Destructor(Arc<Stage>);
impl Drop for Destructor {
    fn drop(&mut self) {
        self.0.wait();
    }
}
/// Every exit releases both one-shot stages, then observes actual thread join
/// before its containing test can drop the private runtime directory/context.
struct Original {
    stages: [Arc<Stage>; 2],
    releases: [Option<SyncSender<()>>; 2],
    completion: Option<Completion>,
}
impl Original {
    fn new() -> Self {
        let (initialize, release_initialize) = Stage::new();
        let (destroy, release_destroy) = Stage::new();
        Self {
            stages: [initialize, destroy],
            releases: [Some(release_initialize), Some(release_destroy)],
            completion: None,
        }
    }
    fn release(&mut self, stage: usize) {
        if let Some(release) = self.releases[stage].take() {
            let _ = release.send(());
        }
    }
}
impl Drop for Original {
    fn drop(&mut self) {
        self.release(0);
        self.release(1);
        if let Some(completion) = &self.completion {
            while !completion.is_complete() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
// The initializer always fails before producing storage; these methods retain
// no input, allocation or platform resource and are never used for persistence.
struct NeverInitialized;
impl TrustStorage for NeverInitialized {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    fn replace(&mut self, _record: &[u8]) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn t1100_withdrawn_initializer_and_destructor_keep_exact_admission_until_join() {
    let root = tempfile::tempdir().unwrap();
    let pool = Pool::new(1).unwrap();
    let mut original = Original::new();
    let initialize = original.stages[0].clone();
    let destroy = original.stages[1].clone();
    let worker = Worker::start_reserved::<NeverInitialized>(pool.reserve().unwrap(), move || {
        let _destructor = Destructor(destroy);
        initialize.wait();
        anyhow::bail!("T1100 partial initialization failure")
    })
    .unwrap();
    let completion = worker.completion();
    original.completion = Some(completion.clone());
    drop(worker);
    let settings = Settings::new(
        root.path().join("runtime"),
        root.path().join("state"),
        root.path().join("helper"),
    );
    for stage in 0..2 {
        bounded(async {
            while !original.stages[stage].entered.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await;
        assert!(!completion.is_complete());
        assert_eq!(pool.active(), 1);
        assert!(Server::open_in(settings.clone(), Some(&pool))
            .await
            .is_err());
        assert!(!settings.runtime.exists());
        assert!(!settings.state.exists());
        original.release(stage);
    }
    bounded(async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert_eq!(pool.active(), 0);
    let next = pool.reserve().unwrap();
    assert_eq!(pool.active(), 1);
    drop(next);
}
