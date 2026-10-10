//! T814: control deadlines must not wait for an already committing filesystem call.
use super::*;
use std::{
    sync::{mpsc as sync, Mutex},
    time::Duration,
};
struct Held {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<sync::Receiver<()>>,
}
impl Transactions for Held {
    fn transact(
        &self,
        _: &AtomicBool,
        edit: blent_config::persistence_contract::Edit<'_>,
    ) -> Result<FileConfig> {
        self.entered
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(())
            .unwrap();
        self.release.lock().unwrap().recv().unwrap();
        let mut config = FileConfig::default();
        edit(&mut config)?;
        Ok(config)
    }
}
#[tokio::test]
async fn t814_held_commit_does_not_hold_control_shutdown() {
    let (entered, started) = oneshot::channel();
    let (release, hold) = sync::channel();
    let pool = blent::control_workers::WorkerPool::new(1).unwrap();
    let worker = Worker::with_pool(
        Held {
            entered: Mutex::new(Some(entered)),
            release: Mutex::new(hold),
        },
        pool.clone(),
    )
    .unwrap();
    let writer = worker.writer();
    let write = tokio::spawn(async move {
        writer
            .update(|config| {
                config.fps = 30;
                Ok(())
            })
            .await
    });
    started.await.unwrap();
    let stopped = tokio::time::timeout(Duration::from_secs(2), worker.shutdown()).await;
    let pending = stopped.as_ref().is_ok_and(|ticket| !ticket.is_complete());
    let occupied = pool.active();
    let replacement_rejected =
        blent::control_workers::Worker::spawn(pool.clone(), "T814 replacement", || {}).is_err();
    // Always release the fixture, including the pre-fix failure path.
    release.send(()).unwrap();
    assert_eq!(
        write.await.unwrap().unwrap().fps,
        30,
        "T814: in-progress transaction lost its result"
    );
    assert!(pending && occupied == 1 && replacement_rejected);
    let ticket = stopped.as_ref().unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !ticket.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(pool.active(), 0);
    assert!(
        stopped.is_ok(),
        "T814: control shutdown waited for held filesystem work"
    );
}
