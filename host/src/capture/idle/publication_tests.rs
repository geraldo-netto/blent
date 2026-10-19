//! T1115: observe actual publication boundaries, independently of getter counts.
use super::*;
use blent::capture_contract::LivenessCapability;
use std::sync::mpsc;
use tokio::sync::watch;

pub(super) struct Hook {
    entered: mpsc::Sender<u8>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Hook {
    fn step(&self, phase: u8) {
        if self.entered.send(phase).is_ok() {
            let _ = self.release.lock().unwrap().recv();
        }
    }
}
pub(super) struct Publication(Option<Arc<Hook>>);
impl Publication {
    pub(super) fn new(hook: Option<Arc<Hook>>) -> Self {
        if let Some(hook) = &hook {
            hook.step(1);
        }
        Self(hook)
    }
}
impl Drop for Publication {
    fn drop(&mut self) {
        if let Some(hook) = &self.0 {
            hook.step(2);
        }
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
        let _ = self.0.send(());
    }
}
struct Grant(watch::Receiver<bool>);
impl LivenessCapability for Grant {
    fn supported(&self) -> bool {
        self.0.has_changed().is_ok() && *self.0.borrow()
    }
    fn changed(&self) -> blent::capture_contract::Signal<'static> {
        let mut source = self.0.clone();
        let open = source.has_changed().is_ok();
        source.borrow_and_update();
        Box::pin(async move {
            if open {
                let _ = source.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        })
    }
}
async fn stage(entered: &mpsc::Receiver<u8>, expected: u8) {
    let mut observed = None;
    super::tests::native_until(|| {
        observed = entered.try_recv().ok();
        observed.is_some()
    })
    .await;
    assert_eq!(observed, Some(expected));
}

async fn publication_wait_case(expire: bool) {
    let (grant, source) = watch::channel(true);
    let trial =
        super::trial_tests::Trial::with_capability("libx264", Arc::new(Grant(source)), None).await;
    let (release, resumed) = mpsc::channel();
    // Declared after Trial: failure/cancel releases native hooks before membership
    // destruction can wait on the original native worker's borrowed view.
    let release = Release(release);
    let (notify, entered) = mpsc::channel();
    let hook = Arc::new(Hook {
        entered: notify,
        release: Mutex::new(resumed),
    });
    trial.qualify().await;
    *trial.handle()._lease.publication_observer.lock().unwrap() = Some(hook);
    tokio::time::advance(Duration::from_millis(250)).await;
    stage(&entered, 1).await;
    let directory =
        directory::Directory::open(trial.handle()._lease.path.parent().unwrap()).unwrap();
    let held = directory.lock().unwrap();
    std::fs::remove_file(&trial.handle()._lease.path).unwrap();
    if expire {
        tokio::time::advance(Duration::from_millis(1501)).await;
    } else {
        drop(grant);
    }
    release.0.send(()).unwrap();
    drop(held);
    stage(&entered, 2).await;
    let stale_publication = trial.handle()._lease.path.exists();
    release.0.send(()).unwrap();
    trial.cleared().await;
    trial.finish().await;
    assert!(
        !stale_publication,
        "T1115: stale original admission published after native wait"
    );
}

#[tokio::test(start_paused = true)]
async fn t1115_liveness_loss_during_original_publication_wait_cannot_publish_stale_advice() {
    if crate::capture::ownership_tests::isolated("capture::idle::publication_tests::t1115_liveness_loss_during_original_publication_wait_cannot_publish_stale_advice") { return; }
    publication_wait_case(false).await;
}

#[tokio::test(start_paused = true)]
async fn t1115_expired_receipt_proof_during_publication_wait_cannot_renew_native_lease() {
    if crate::capture::ownership_tests::isolated("capture::idle::publication_tests::t1115_expired_receipt_proof_during_publication_wait_cannot_renew_native_lease") { return; }
    publication_wait_case(true).await;
}
