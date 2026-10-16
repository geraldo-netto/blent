//! Global calibration ownership is independent of its parent future's result.
use blent::native_scope::Scope;
use blent_config::retirement::Pool;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(super) struct Calibration {
    scope: Option<Scope>,
    lease: Option<Lease>,
}
impl Calibration {
    pub(super) fn scope(&self) -> &Scope {
        self.scope.as_ref().unwrap()
    }
}
impl Drop for Calibration {
    fn drop(&mut self) {
        drop(self.scope.take());
        self.lease.take().unwrap().retire();
    }
}
struct Lease {
    permit: OwnedSemaphorePermit,
    native: blent::native_scope::Retirement,
    slot: blent_config::retirement::Admission,
}
impl Lease {
    fn retire(self) {
        let Self {
            permit,
            native,
            slot,
        } = self;
        if native.is_complete() {
            drop(permit);
            drop(slot);
        } else {
            slot.retain(Pending {
                _permit: permit,
                native,
            });
        }
    }
}
struct Pending {
    _permit: OwnedSemaphorePermit,
    native: blent::native_scope::Retirement,
}
impl blent_config::retirement::Observe for Pending {
    fn poll_complete(&mut self) -> bool {
        self.native.is_complete()
    }
}
async fn reserve(pool: &Pool) -> std::io::Result<blent_config::retirement::Admission> {
    loop {
        match pool.reserve() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
            result => return result,
        }
    }
}
async fn begin_with(
    scope: &Scope,
    semaphore: Arc<Semaphore>,
    pool: &Pool,
) -> std::io::Result<Calibration> {
    let permit = semaphore
        .acquire_owned()
        .await
        .map_err(std::io::Error::other)?;
    let slot = reserve(pool).await?;
    let (scoped, native) = scope.group();
    scope.retain(slot.completion());
    Ok(Calibration {
        scope: Some(scoped),
        lease: Some(Lease {
            permit,
            native,
            slot,
        }),
    })
}

fn pool() -> std::io::Result<&'static Pool> {
    static POOL: std::sync::OnceLock<std::io::Result<Pool>> = std::sync::OnceLock::new();
    POOL.get_or_init(|| Pool::new(1))
        .as_ref()
        .map_err(|error| std::io::Error::new(error.kind(), error.to_string()))
}
pub(super) async fn begin(scope: &Scope) -> Option<Calibration> {
    bounded(scope, super::ADMISSION.clone(), pool().ok()?).await
}
async fn bounded(scope: &Scope, semaphore: Arc<Semaphore>, pool: &Pool) -> Option<Calibration> {
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        begin_with(scope, semaphore, pool),
    )
    .await
    .ok()?
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{mpsc, Mutex},
        time::Duration,
    };
    struct Held(mpsc::Sender<()>, Mutex<mpsc::Receiver<()>>);
    impl Drop for Held {
        fn drop(&mut self) {
            let _ = self.0.send(());
            let _ = self.1.lock().unwrap().recv();
        }
    }
    struct Release(mpsc::Sender<()>);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    async fn released(semaphore: &Semaphore) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if semaphore.try_acquire().is_ok() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn t1033_canceled_calibration_keeps_actual_child_cleanup_admission() {
        let semaphore = Arc::new(Semaphore::new(1));
        let pool = Pool::new(1).unwrap();
        let calibration = begin_with(&Scope::default(), semaphore.clone(), &pool)
            .await
            .unwrap();
        let mut command = tokio::process::Command::new("/bin/sleep");
        command.arg("60");
        let mut child = calibration.scope().spawn(&mut command).unwrap();
        let original = child.completion();
        let (entered_tx, entered) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        child.keep_alive(Held(entered_tx, Mutex::new(held)));
        drop(child);
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(calibration);
        let retained = semaphore.try_acquire().is_err();
        assert!(!original.is_complete());
        drop(release);
        released(&semaphore).await;
        assert!(
            retained,
            "T1033 calibration permit released during original native cleanup"
        );
    }
    #[tokio::test]
    async fn t1033_canceled_calibration_keeps_actual_blocking_creator_admission() {
        let semaphore = Arc::new(Semaphore::new(1));
        let pool = Pool::new(1).unwrap();
        let calibration = begin_with(&Scope::default(), semaphore.clone(), &pool)
            .await
            .unwrap();
        let scope = calibration.scope().clone();
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        let (exited_tx, mut exited) = tokio::sync::oneshot::channel();
        let work = tokio::spawn(async move {
            scope
                .blocking(move |scope| {
                    drop(scope);
                    entered_tx.send(()).unwrap();
                    held.recv().unwrap();
                    exited_tx.send(()).unwrap();
                })
                .await
        });
        entered.await.unwrap();
        work.abort();
        assert!(work.await.unwrap_err().is_cancelled());
        drop(calibration);
        let retained = semaphore.try_acquire().is_err();
        assert!(exited.try_recv().is_err());
        drop(release);
        exited.await.unwrap();
        released(&semaphore).await;
        assert!(
            retained,
            "T1033 calibration permit released during original creator cleanup"
        );
    }
    #[derive(Default)]
    struct Report(Mutex<Vec<blent_config::retirement::Completion>>);
    impl blent::capture_contract::NativeRetirement for Report {
        fn retain(&self, proof: blent_config::retirement::Completion) {
            self.0.lock().unwrap().push(proof);
        }
    }
    async fn completed(proof: &blent_config::retirement::Completion) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !proof.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn t1033_late_child_keeps_admission_after_canceled_creator_joins() {
        let semaphore = Arc::new(Semaphore::new(1));
        let pool = Pool::new(1).unwrap();
        let report = Arc::new(Report::default());
        let parent = Scope::new(Some(report.clone()));
        let calibration = begin_with(&parent, semaphore.clone(), &pool).await.unwrap();
        let scope = calibration.scope().clone();
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (create, blocked) = mpsc::channel();
        let create = Release(create);
        let (cleanup_tx, cleanup) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let release = Release(release);
        let (child_tx, child) = tokio::sync::oneshot::channel();
        let work = tokio::spawn(async move {
            scope
                .blocking(move |scope| {
                    entered_tx.send(()).unwrap();
                    blocked.recv().unwrap();
                    let mut command = tokio::process::Command::new("/bin/sleep");
                    command.arg("60");
                    let mut child = scope.spawn(&mut command).unwrap();
                    child.keep_alive(Held(cleanup_tx, Mutex::new(held)));
                    child_tx.send(child.completion()).unwrap();
                    drop(child);
                })
                .await
        });
        entered.await.unwrap();
        let creator = {
            let originals = report.0.lock().unwrap();
            assert_eq!(originals.len(), 2);
            originals[1].clone()
        };
        work.abort();
        assert!(work.await.unwrap_err().is_cancelled());
        drop(calibration);
        let held_before = semaphore.try_acquire().is_err();
        drop(create);
        let child = child.await.unwrap();
        completed(&creator).await;
        cleanup.recv_timeout(Duration::from_secs(5)).unwrap();
        let held_after = semaphore.try_acquire().is_err();
        assert!(!child.is_complete());
        let sibling = Arc::new(Semaphore::new(1));
        drop(
            begin_with(&Scope::default(), sibling.clone(), &Pool::new(1).unwrap())
                .await
                .unwrap(),
        );
        assert!(
            sibling.try_acquire().is_ok(),
            "T1033 unrelated admission stalled"
        );
        drop(release);
        completed(&child).await;
        released(&semaphore).await;
        assert!(
            held_before && held_after,
            "T1033 late original child escaped calibration admission"
        );
    }
    #[tokio::test(start_paused = true)]
    async fn t1033_original_admission_deadline_and_closed_source_reject_without_work() {
        let semaphore = Arc::new(Semaphore::new(0));
        let pool = Pool::new(1).unwrap();
        let start = tokio::time::Instant::now();
        assert!(bounded(&Scope::default(), semaphore.clone(), &pool)
            .await
            .is_none());
        assert_eq!(start.elapsed(), Duration::from_secs(3));
        semaphore.close();
        assert!(bounded(&Scope::default(), semaphore, &pool).await.is_none());
        assert_eq!(pool.active(), 0);
    }
    #[tokio::test(start_paused = true)]
    async fn t1033_pending_retirement_budget_keeps_same_admission_deadline() {
        let semaphore = Arc::new(Semaphore::new(1));
        let pool = Pool::new(1).unwrap();
        let pending = pool.reserve().unwrap();
        let start = tokio::time::Instant::now();
        assert!(bounded(&Scope::default(), semaphore.clone(), &pool)
            .await
            .is_none());
        assert_eq!(start.elapsed(), Duration::from_secs(3));
        assert!(semaphore.try_acquire().is_ok());
        drop(pending);
        drop(
            bounded(&Scope::default(), semaphore.clone(), &pool)
                .await
                .unwrap(),
        );
        assert!(semaphore.try_acquire().is_ok());
    }
}
