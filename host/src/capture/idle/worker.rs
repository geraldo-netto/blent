//! Native lease effects never execute on Tokio control workers.
use super::Lease;
use blent_config::retirement::{Admission, Completion, Observe};
use blent_config::retirement_work::{Cleanup, Thread};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
pub(super) struct Stop {
    revoked: AtomicBool,
    wake: tokio::sync::Notify,
}
impl Stop {
    pub(super) fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
        self.wake.notify_one();
    }
    pub(super) fn active(&self) -> bool {
        !self.revoked.load(Ordering::Acquire)
    }
    pub(super) async fn changed(&self) {
        self.wake.notified().await;
    }
}
type Held = Arc<Mutex<Option<Arc<Lease>>>>;
pub(super) struct Holder(Held);
impl Holder {
    pub(super) fn retain(&self, lease: Arc<Lease>) -> anyhow::Result<()> {
        let mut held = self.0.lock().unwrap();
        anyhow::ensure!(held.is_none(), "native idle lease already retained");
        *held = Some(lease);
        Ok(())
    }
}
pub(super) struct Worker {
    stop: Arc<Stop>,
    thread: Option<std::thread::JoinHandle<()>>,
    admission: Option<Admission>,
    held: Held,
}
impl Worker {
    pub(super) fn spawn(
        stop: Arc<Stop>,
        scope: &blent::native_scope::Scope,
        work: impl FnOnce(Holder) -> anyhow::Result<()> + Send + 'static,
    ) -> std::io::Result<Self> {
        let admission = blent::work_admission::reserve()?;
        let held = Held::default();
        let holder = Holder(held.clone());
        let thread = std::thread::Builder::new()
            .name("blent-idle".into())
            .spawn(move || {
                let outcome =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(holder)));
                match outcome {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => tracing::warn!(%error, "Native idle unavailable"),
                    Err(_) => tracing::error!(
                        "Native idle worker panicked; original cleanup remains retained"
                    ),
                }
            })?;
        let worker = Self {
            stop,
            thread: Some(thread),
            admission: Some(admission),
            held,
        };
        scope.retain(worker.completion());
        Ok(worker)
    }
    pub(super) fn completion(&self) -> Completion {
        self.admission.as_ref().unwrap().completion()
    }
    // Test observation only; thread exit does not prove native lease retirement.
    #[cfg(test)]
    pub(super) fn is_finished(&self) -> bool {
        self.thread.as_ref().unwrap().is_finished()
    }
    pub(super) fn abort(&self) {
        self.stop.revoke();
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.abort();
        let thread = Thread::new(self.thread.take().unwrap());
        self.admission.take().unwrap().retain(Retiring {
            thread,
            held: self.held.clone(),
            cleanup: None,
        });
    }
}
struct Retiring {
    thread: Thread,
    held: Held,
    cleanup: Option<Cleanup>,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        if !self.thread.poll_complete() {
            return false;
        }
        if let Some(cleanup) = &mut self.cleanup {
            return cleanup.poll_complete();
        }
        let Some(lease) = self.held.lock().unwrap().take() else {
            return true;
        };
        self.cleanup = Some(Cleanup::new(move || drop(lease)));
        false
    }
}
