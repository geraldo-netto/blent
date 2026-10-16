//! T1024: logical revocation precedes asynchronous cancellation and replacement.
use super::*;
use crate::capture::ownership_tests::isolated;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;

async fn completed(original: &blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1024_late_original_reports_cannot_replace_current_publication() {
    let mut helper = HelperProcess::new();
    let publisher = helper.stdout_publisher();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let task = blent::owned_task::Task::spawn_blocking(move || {
        entered.send(()).unwrap();
        held.recv_timeout(Duration::from_secs(5)).unwrap();
        HelperProcess::publish_helper_line("STREAM_SIZE 32 32", &publisher);
        HelperProcess::publish_helper_line("MODE_CHANGED 32 32 30", &publisher);
        publisher.fifo(fifo::Identity::from_reset_line("FIFO_RESET 1 2").unwrap());
        Ok(())
    })
    .unwrap();
    let original = task.completion();
    helper.stdout_task = Some(task);
    ready.await.unwrap();
    helper.abort_stdout();
    assert!(!original.is_complete());
    let current = DetectedMode {
        width: 640,
        height: 480,
        refresh: 60,
    };
    helper.mode_tx.send(Some(current)).unwrap();
    helper.stream_tx.send(Some((640, 480))).unwrap();
    let identity = fifo::Identity::from_reset_line("FIFO_RESET 3 4");
    helper.fifo_reset_tx.send(identity).unwrap();
    release.send(()).unwrap();
    completed(&original).await;
    assert_eq!(
        *helper.mode_rx.borrow(),
        Some(current),
        "T1024 late mode crossed original stop"
    );
    assert_eq!(
        *helper.stream_rx.borrow(),
        Some((640, 480)),
        "T1024 late stream crossed original stop"
    );
    assert_eq!(
        *helper.fifo_reset_rx.borrow(),
        identity,
        "T1024 late FIFO crossed original stop"
    );
}

#[tokio::test]
async fn t1024_held_original_stdout_blocks_replacement_before_spawn() {
    if isolated("capture::helper::publication_tests::t1024_held_original_stdout_blocks_replacement_before_spawn") { return; }
    let config = super::startup_ownership_tests::configuration();
    let mut helper = HelperProcess::new();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = mpsc::channel();
    let task = blent::owned_task::Task::spawn_blocking(move || {
        entered.send(()).unwrap();
        held.recv_timeout(Duration::from_secs(5)).unwrap();
        Ok(())
    })
    .unwrap();
    let original = task.completion();
    helper.stdout_task = Some(task);
    ready.await.unwrap();
    let spawns = Arc::new(AtomicUsize::new(0));
    let owned_count = spawns.clone();
    let owned_result = helper
        .start_with(&config, |_| {
            owned_count.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("T1024 forbidden owned stdout spawn"))
        })
        .await;
    helper.abort_stdout();
    helper.terminate().await;
    assert!(!original.is_complete());
    let count = spawns.clone();
    let result = helper
        .start_with(&config, |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("T1024 forbidden replacement spawn"))
        })
        .await;
    release.send(()).unwrap();
    completed(&original).await;
    assert!(owned_result.is_err());
    assert!(result.is_err());
    assert_eq!(
        spawns.load(Ordering::SeqCst),
        0,
        "T1024 replacement bypassed original stdout cleanup"
    );
    helper.stdout_failure = Some("T1022 original transport failed".into());
    helper.start(&config).await.unwrap();
    assert!(helper.stdout_failure.is_none());
    assert_eq!(helper.card, Some(17));
    helper.terminate().await;
}

#[tokio::test]
async fn t1024_live_helper_reentry_preserves_original_child_and_fifo() {
    if isolated("capture::helper::publication_tests::t1024_live_helper_reentry_preserves_original_child_and_fifo") { return; }
    use std::os::unix::fs::MetadataExt;
    let config = super::startup_ownership_tests::configuration();
    let mut helper = HelperProcess::new();
    helper.start(&config).await.unwrap();
    let original = helper.child.as_ref().unwrap().completion();
    let path = fifo_path_for(config.instance).unwrap();
    let inode = std::fs::symlink_metadata(&path).unwrap();
    let spawns = Arc::new(AtomicUsize::new(0));
    let count = spawns.clone();
    let result = helper
        .start_with(&config, |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("T1024 forbidden live spawn"))
        })
        .await;
    let retained = std::fs::symlink_metadata(&path).unwrap();
    let running = helper.child.as_mut().unwrap().try_wait().unwrap().is_none();
    assert!(!original.is_complete());
    helper.terminate().await;
    assert!(result.is_err());
    assert_eq!(
        spawns.load(Ordering::SeqCst),
        0,
        "T1024 live helper accepted a replacement spawn"
    );
    assert!(running);
    assert_eq!((retained.dev(), retained.ino()), (inode.dev(), inode.ino()));
}

struct JoinedLease {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Drop for JoinedLease {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
}

#[tokio::test]
async fn t1030_joined_stdout_keeps_original_cleanup_before_helper_restart() {
    if isolated("capture::helper::publication_tests::t1030_joined_stdout_keeps_original_cleanup_before_helper_restart") { return; }
    let config = super::startup_ownership_tests::configuration();
    let mut helper = HelperProcess::new();
    helper.child = Some(Child::spawn(Command::new("/bin/sleep").arg("60")).unwrap());
    let (entered, waiting) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let mut task = blent::owned_task::Task::spawn(async { Ok(()) }).unwrap();
    task.keep_alive(JoinedLease {
        entered,
        release: Mutex::new(held),
    });
    let original = task.completion();
    helper.stdout_task = Some(task);
    assert!(helper.wait().await.is_err());
    waiting.recv_timeout(Duration::from_secs(3)).unwrap();
    helper.terminate().await;
    let spawns = Arc::new(AtomicUsize::new(0));
    let count = spawns.clone();
    let result = helper
        .start_with(&config, |_| {
            count.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("T1030 forbidden helper restart"))
        })
        .await;
    let retained = !original.is_complete();
    release.send(()).unwrap();
    completed(&original).await;
    assert!(result.is_err());
    assert!(retained);
    assert_eq!(
        spawns.load(Ordering::SeqCst),
        0,
        "T1030 joined stdout lost original cleanup before restart"
    );
    helper.start(&config).await.unwrap();
    helper.terminate().await;
}
