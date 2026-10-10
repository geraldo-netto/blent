//! Isolated cleanup work owned by an already-admitted retirement record.
//! Never create this outside a retained resource budget: one monitor is allowed
//! per record, and its admission must remain held until poll_complete returns true.
use crate::retirement::Observe;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
struct Shared {
    work: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    done: AtomicBool,
}
pub struct Cleanup {
    shared: Arc<Shared>,
    started: bool,
    next_try: Instant,
}
impl Cleanup {
    pub fn new(work: impl FnOnce() + Send + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                work: Mutex::new(Some(Box::new(work))),
                done: AtomicBool::new(false),
            }),
            started: false,
            next_try: Instant::now(),
        }
    }
    fn start(&mut self) {
        if self.started || Instant::now() < self.next_try {
            return;
        }
        self.next_try = Instant::now() + Duration::from_secs(1);
        let shared = self.shared.clone();
        // On spawn failure the original shared work stays owned, and the reaper
        // retries at most once per second. No native handle is detached/lost.
        self.started = std::thread::Builder::new()
            .name("blent-retire-work".into())
            .spawn(move || {
                let work = shared.work.lock().unwrap().take().unwrap();
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_ok() {
                    shared.done.store(true, Ordering::Release);
                } else {
                    tracing::error!("Resource cleanup panicked; admission remains quarantined");
                }
            })
            .is_ok();
    }
}
impl Observe for Cleanup {
    fn poll_complete(&mut self) -> bool {
        self.start();
        self.shared.done.load(Ordering::Acquire)
    }
}

/// Observe a native worker without joining on the shared retirement thread.
/// Native TLS teardown may still be running after is_finished becomes true.
pub struct Thread {
    handle: Option<std::thread::JoinHandle<()>>,
    cleanup: Option<Cleanup>,
}
impl Thread {
    pub fn new(handle: std::thread::JoinHandle<()>) -> Self {
        Self {
            handle: Some(handle),
            cleanup: None,
        }
    }
}
impl Observe for Thread {
    fn poll_complete(&mut self) -> bool {
        if let Some(handle) = &self.handle {
            if !handle.is_finished() {
                return false;
            }
            let handle = self.handle.take().unwrap();
            self.cleanup = Some(Cleanup::new(move || {
                let _ = handle.join();
            }));
        }
        self.cleanup.as_mut().unwrap().poll_complete()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::retirement::Pool;
    #[test]
    fn t814_cleanup_panic_cannot_release_unknown_resources_or_block_siblings() {
        let pool = Pool::new(2).unwrap();
        let slot = pool.reserve().unwrap();
        let failed = slot.completion();
        slot.retain(Cleanup::new(|| panic!("T814 destructor panic")));
        let slot = pool.reserve().unwrap();
        let good = slot.completion();
        slot.retain(Cleanup::new(|| {}));
        let end = Instant::now() + Duration::from_secs(2);
        while !good.is_complete() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!failed.is_complete());
        assert_eq!(pool.active(), 1);
    }
}
