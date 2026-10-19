//! T1125: the completed future itself must release its original native guard.
use super::*;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

struct OriginalFuture {
    owner: Arc<Mutex<Option<blent_config::retirement::Admission>>>,
    outcome: u8,
}
impl Future for OriginalFuture {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(result(self.outcome))
    }
}
impl Drop for OriginalFuture {
    fn drop(&mut self) {
        self.owner.lock().unwrap().take();
    }
}

#[test]
fn t1125_original_future_drops_before_its_own_native_settlement_wait() {
    for outcome in 0..3 {
        original_future(outcome);
    }
}
fn original_future(outcome: u8) {
    let original = Pool::new(1).unwrap().reserve().unwrap();
    let proof = original.completion();
    let owner = Arc::new(Mutex::new(Some(original)));
    let retained = owner.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let entered = calls.clone();
    let controller = Controller::with_pool(
        crate::control_workers::WorkerPool::new(1).unwrap(),
        move |_, _, report| {
            report
                .reserve_native(native::Resource::Connection)
                .unwrap()
                .bind(proof.clone());
            entered.fetch_add(1, Ordering::SeqCst);
            OriginalFuture {
                owner: retained.clone(),
                outcome,
            }
        },
    )
    .unwrap();
    controller.start(CameraProfile::default()).unwrap();
    until(|| calls.load(Ordering::SeqCst) == 1);
    let end = Instant::now() + Duration::from_secs(2);
    while owner.lock().unwrap().is_some() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(1));
    }
    let released = owner.lock().unwrap().is_none();
    // The fixture sponsors cleanup even if the future retained its own guard.
    owner.lock().unwrap().take();
    let complete = controller.shutdown();
    until(|| complete.is_complete());
    assert!(
        released,
        "T1125 completed future retained the guard it awaited"
    );
}
