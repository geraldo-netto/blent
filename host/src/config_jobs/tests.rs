use super::*;
use std::sync::Mutex;

struct Memory {
    value: Mutex<FileConfig>,
    entered: std::sync::mpsc::Sender<()>,
    hold: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl Snapshots for Memory {
    fn snapshot(&self) -> FileConfig {
        self.entered.send(()).unwrap();
        self.hold.lock().unwrap().recv().unwrap();
        self.value.lock().unwrap().clone()
    }
}
impl Transactions for Memory {
    fn transact(&self, cancelled: &AtomicBool, edit: Edit<'_>) -> Result<FileConfig> {
        anyhow::ensure!(!cancelled.load(Ordering::Acquire), "cancelled");
        let mut value = self.value.lock().unwrap();
        edit(&mut value)?;
        Ok(value.clone())
    }
}
#[tokio::test]
async fn t826_held_store_bounds_queue_cancels_queued_work_and_retains_worker() {
    let pool = WorkerPool::new(1).unwrap();
    let (entered, entry) = std::sync::mpsc::channel();
    let (release, hold) = std::sync::mpsc::channel();
    let store = Store::with_pool(
        Memory {
            value: Mutex::new(FileConfig::default()),
            entered,
            hold: Mutex::new(hold),
        },
        pool.clone(),
    )
    .unwrap();
    {
        let first = store.snapshot();
        tokio::pin!(first);
        assert!(futures_util::poll!(&mut first).is_pending());
        tokio::task::spawn_blocking(move || {
            entry
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
        })
        .await
        .unwrap();
        {
            let queued = store.transact(Box::new(|_| panic!("cancelled queued edit ran")));
            tokio::pin!(queued);
            assert!(futures_util::poll!(&mut queued).is_pending());
            assert!(store.snapshot().await.is_err());
        }
        release.send(()).unwrap();
        assert_eq!(first.as_mut().await.unwrap(), FileConfig::default());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match store
                .transact(Box::new(|config| {
                    config.fps = 30;
                    Ok(())
                }))
                .await
            {
                Ok(value) => {
                    assert_eq!(value.fps, 30);
                    break;
                }
                Err(_) => {
                    assert!(tokio::time::Instant::now() < deadline);
                    tokio::task::yield_now().await;
                }
            }
        }
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    drop(store);
    while pool.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
#[test]
fn t826_worker_capacity_rejects_invalid_or_nonunicode_configuration() {
    for text in ["", "-1", "0", "65", "999999999999999999999999", "NaN", " 2"] {
        assert!(configured_pool(Ok(text.into())).is_err());
    }
    assert!(configured_pool(Ok("1".into())).is_ok());
    assert!(configured_pool(Err(std::env::VarError::NotPresent)).is_ok());
    assert!(configured_pool(Err(std::env::VarError::NotUnicode("invalid".into()))).is_err());
}

#[tokio::test]
async fn t826_abandoned_active_read_retains_capacity_until_native_exit() {
    let pool = WorkerPool::new(1).unwrap();
    let (entered, entry) = std::sync::mpsc::channel();
    let (release, hold) = std::sync::mpsc::channel();
    let store = Store::with_pool(
        Memory {
            value: Mutex::new(FileConfig::default()),
            entered,
            hold: Mutex::new(hold),
        },
        pool.clone(),
    )
    .unwrap();
    {
        let read = store.snapshot();
        tokio::pin!(read);
        assert!(futures_util::poll!(&mut read).is_pending());
        tokio::task::spawn_blocking(move || {
            entry
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
        })
        .await
        .unwrap();
    }
    drop(store);
    assert_eq!(pool.active(), 1);
    assert!(Worker::spawn(pool.clone(), "excess", || {}).is_err());
    release.send(()).unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while pool.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}

#[tokio::test]
async fn t826_cancelled_lock_wait_never_commits_and_worker_failure_is_observed() {
    let root = tempfile::tempdir().unwrap();
    let native = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    let initial = native.update(|_| Ok(())).unwrap();
    let lock = std::fs::File::open(native.path().unwrap().with_extension("lock")).unwrap();
    lock.lock().unwrap();
    let pool = WorkerPool::new(1).unwrap();
    let store = Store::with_pool(native.clone(), pool.clone()).unwrap();
    assert!(tokio::time::timeout(
        std::time::Duration::from_millis(30),
        store.transact(Box::new(|config| {
            config.fps = 30;
            Ok(())
        }))
    )
    .await
    .is_err());
    drop(store);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while pool.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    assert_eq!(native.load(), initial);
    drop(lock);
    struct Broken;
    impl Snapshots for Broken {
        fn snapshot(&self) -> FileConfig {
            panic!("broken snapshot")
        }
    }
    impl Transactions for Broken {
        fn transact(&self, _: &AtomicBool, _: Edit<'_>) -> Result<FileConfig> {
            anyhow::bail!("unavailable")
        }
    }
    let store = Store::with_pool(Broken, pool).unwrap();
    assert!(store
        .snapshot()
        .await
        .unwrap_err()
        .to_string()
        .contains("failed"));
    assert!(store.transact(Box::new(|_| Ok(()))).await.is_err());
}

#[tokio::test]
async fn t826_reusable_sibling_progress_and_partial_construction_retirement() {
    let pool = WorkerPool::new(2).unwrap();
    let executor = Executor::new(pool.clone(), 2).unwrap();
    let (entered, entry) = mpsc::channel();
    let (release, hold) = mpsc::channel();
    let blocked = Store::on(
        Memory {
            value: Mutex::new(FileConfig::default()),
            entered,
            hold: Mutex::new(hold),
        },
        executor.clone(),
    );
    let root = tempfile::tempdir().unwrap();
    let healthy = Store::on(
        blent_config::storage::ConfigStore::new(root.path().join("config.toml")),
        executor.clone(),
    );
    {
        let read = blocked.snapshot();
        tokio::pin!(read);
        assert!(futures_util::poll!(&mut read).is_pending());
        tokio::task::spawn_blocking(move || {
            entry
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
        })
        .await
        .unwrap();
        for value in 10..40 {
            let config = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                healthy.transact(Box::new(move |config| {
                    config.fps = value;
                    Ok(())
                })),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(config.fps, value);
            assert_eq!(healthy.snapshot().await.unwrap().fps, value);
            assert_eq!(pool.active(), 2, "T826: native workers must be reused");
        }
        release.send(()).unwrap();
        read.await.unwrap();
    }
    drop((blocked, healthy, executor));
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while pool.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    let small = WorkerPool::new(1).unwrap();
    for count in [0, 65, usize::MAX] {
        assert!(Executor::new(small.clone(), count).is_err());
    }
    assert!(Executor::new(small.clone(), 2).is_err());
    while small.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}

#[tokio::test]
async fn t826_cancellation_during_commit_retains_real_outcome() {
    struct Commit {
        value: Arc<Mutex<FileConfig>>,
        entered: mpsc::Sender<()>,
        hold: Mutex<mpsc::Receiver<()>>,
    }
    impl Snapshots for Commit {
        fn snapshot(&self) -> FileConfig {
            self.value.lock().unwrap().clone()
        }
    }
    impl Transactions for Commit {
        fn transact(&self, _: &AtomicBool, edit: Edit<'_>) -> Result<FileConfig> {
            let mut value = self.value.lock().unwrap();
            edit(&mut value)?;
            self.entered.send(()).unwrap();
            self.hold.lock().unwrap().recv().unwrap();
            Ok(value.clone())
        }
    }
    let value = Arc::new(Mutex::new(FileConfig::default()));
    let (entered, entry) = mpsc::channel();
    let (release, hold) = mpsc::channel();
    let pool = WorkerPool::new(1).unwrap();
    let store = Store::with_pool(
        Commit {
            value: value.clone(),
            entered,
            hold: Mutex::new(hold),
        },
        pool.clone(),
    )
    .unwrap();
    {
        let edit = store.transact(Box::new(|config| {
            config.fps = 30;
            Ok(())
        }));
        tokio::pin!(edit);
        assert!(futures_util::poll!(&mut edit).is_pending());
        tokio::task::spawn_blocking(move || {
            entry
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
        })
        .await
        .unwrap();
    }
    drop(store);
    assert_eq!(pool.active(), 1);
    release.send(()).unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while pool.active() != 0 {
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    assert_eq!(
        value.lock().unwrap().fps,
        30,
        "T826: cancellation cannot roll back an in-progress commit"
    );
}
