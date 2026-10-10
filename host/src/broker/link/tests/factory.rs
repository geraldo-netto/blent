use super::*;
use blent_config::retirement::Observe;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};

#[derive(Default)]
struct Gate {
    entered: AtomicBool,
    open: Mutex<bool>,
    wake: Condvar,
}
impl Gate {
    fn wait(&self) {
        self.entered.store(true, Ordering::Release);
        drop(
            self.wake
                .wait_while(self.open.lock().unwrap(), |open| !*open)
                .unwrap(),
        );
    }
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t853_cancelled_blocking_factory_keeps_capacity_until_actual_return() {
    let pool = Pool::new(1).unwrap();
    let gate = Arc::new(Gate::default());
    let _release = Release(gate.clone());
    let blocked = gate.clone();
    let (native, _remote) = resource();
    let inner = native.owner.completion();
    let mut handle = Handle::start_factory(
        &pool,
        move || {
            blocked.wait();
            Ok(native)
        },
        settings(),
    )
    .unwrap();
    let completion = handle.native_completion();
    until(|| gate.entered.load(Ordering::Acquire)).await;
    handle.stop();
    assert!(tokio::time::timeout(Duration::from_secs(3), handle.next())
        .await
        .unwrap()
        .is_err());
    drop(handle);
    assert!(!completion.is_complete());
    assert!(!inner.is_complete());
    assert!(pool.reserve().is_err());
    assert!(Handle::start_factory(&pool, || Ok(resource().0), settings()).is_err());
    gate.release();
    until(|| completion.is_complete()).await;
    assert!(inner.is_complete());
    assert_eq!(pool.active(), 0);
}
#[tokio::test]
async fn t853_factory_error_releases_startup_admission() {
    let pool = Pool::new(1).unwrap();
    let mut handle = Handle::start_factory(
        &pool,
        || anyhow::bail!("injected spawn failure"),
        settings(),
    )
    .unwrap();
    let completion = handle.native_completion();
    assert!(tokio::time::timeout(Duration::from_secs(3), handle.next())
        .await
        .unwrap()
        .is_err());
    until(|| completion.is_complete()).await;
    assert_eq!(pool.active(), 0);
}
struct Pending(Arc<AtomicBool>);
impl Observe for Pending {
    fn poll_complete(&mut self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
struct Delayed {
    admission: Option<Admission>,
    stopped: Arc<AtomicBool>,
}
impl Drop for Delayed {
    fn drop(&mut self) {
        if let Some(admission) = self.admission.take() {
            admission.retain(Pending(self.stopped.clone()));
        }
    }
}
impl Owner for Delayed {
    fn completion(&self) -> Completion {
        self.admission.as_ref().unwrap().completion()
    }
    fn retire(mut self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            if self.stopped.load(Ordering::SeqCst) {
                self.admission.take();
                Ok(())
            } else {
                anyhow::bail!("native still retiring")
            }
        })
    }
}
fn delayed(stopped: Arc<AtomicBool>) -> Resource {
    let (stream, _remote) = tokio::io::duplex(32);
    Resource {
        stream: Box::new(stream),
        owner: Box::new(Delayed {
            admission: Some(Pool::new(1).unwrap().reserve().unwrap()),
            stopped,
        }),
    }
}
#[tokio::test]
async fn t853_factory_retirement_waits_for_inner_native_completion() {
    for ready in [false, true] {
        let pool = Pool::new(1).unwrap();
        let admission = pool.reserve().unwrap();
        let completion = admission.completion();
        let stopped = Arc::new(AtomicBool::new(ready));
        let created = stopped.clone();
        let owned = start::open(admission, move || Ok(delayed(created))).unwrap();
        assert!(!owned.owner.completion().is_complete());
        drop(owned.stream);
        assert_eq!(owned.owner.retire().await.is_ok(), ready);
        assert_eq!(completion.is_complete(), ready);
        if !ready {
            tokio::time::sleep(Duration::from_millis(15)).await;
            assert!(!completion.is_complete());
            assert!(pool.reserve().is_err());
        }
        stopped.store(true, Ordering::SeqCst);
        until(|| completion.is_complete()).await;
        assert_eq!(pool.active(), 0);
    }
}
