//! T1030: joined results and lease destructor entry are not full retirement.
use super::*;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

struct HeldLease {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Drop for HeldLease {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
    }
}

async fn completed(original: &Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

async fn joined(mode: u8) {
    let pool = Pool::new(1).unwrap();
    let mut task = Task::owned(
        pool.reserve().unwrap(),
        tokio::spawn(async move {
            match mode {
                0 => Ok(()),
                1 => anyhow::bail!("T1030 original task error"),
                _ => panic!("T1030 original task panic"),
            }
        }),
    );
    let (entered, waiting) = mpsc::channel();
    let (release, held) = mpsc::channel();
    task.keep_alive(HeldLease {
        entered,
        release: Mutex::new(held),
    });
    let original = task.completion();
    let outcome = (&mut task).await;
    let retained_at_join = !original.is_complete();
    let dropper = std::thread::spawn(move || drop(task));
    waiting.recv_timeout(Duration::from_secs(3)).unwrap();
    let retained_at_destructor = !original.is_complete();
    let capacity_held = pool.reserve().is_err();
    release.send(()).unwrap();
    dropper.join().unwrap();
    completed(&original).await;
    assert!(
        retained_at_join,
        "T1030 joined result released live dependent leases"
    );
    assert!(
        retained_at_destructor,
        "T1030 destructor entry released original completion"
    );
    assert!(capacity_held);
    match mode {
        0 => outcome.unwrap().unwrap(),
        1 => assert!(outcome.unwrap().is_err()),
        _ => assert!(outcome.unwrap_err().is_panic()),
    }
}

#[tokio::test]
async fn t1030_joined_success_error_and_panic_keep_original_lease_admission() {
    for mode in 0..3 {
        joined(mode).await;
    }
}

#[tokio::test]
async fn t1030_blocked_lease_cleanup_cannot_stall_independent_retirement() {
    let pool = Pool::new(2).unwrap();
    let mut task = Task::owned(pool.reserve().unwrap(), tokio::spawn(async { Ok(()) }));
    let (entered, waiting) = mpsc::channel();
    let (release, held) = mpsc::channel();
    task.keep_alive(HeldLease {
        entered,
        release: Mutex::new(held),
    });
    let original = task.completion();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !task.handle.as_ref().unwrap().is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(task);
    waiting.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(!original.is_complete());
    let sibling = Task::owned(pool.reserve().unwrap(), tokio::spawn(async { Ok(()) }));
    let unaffected = sibling.completion();
    drop(sibling);
    let independent = tokio::time::timeout(Duration::from_millis(500), completed(&unaffected))
        .await
        .is_ok();
    let retained = !original.is_complete();
    release.send(()).unwrap();
    completed(&original).await;
    completed(&unaffected).await;
    assert!(
        independent,
        "T1030 blocked dependency stalled sibling retirement"
    );
    assert!(retained);
    assert_eq!(pool.active(), 0);
}
