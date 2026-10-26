//! Isolated cleanup work owned by an already-admitted retirement record.
//! Never create this outside a retained resource budget: one monitor is allowed
//! per record, and its admission must remain held until poll_complete returns true.
use crate::retirement::Observe;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
mod native;

struct Outcome {
    work: std::thread::Result<()>,
    report: std::thread::Result<()>,
}
struct Shared {
    work: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    outcome: Mutex<Option<Outcome>>,
}
enum Worker {
    Idle,
    Running(native::Join),
    Joined,
}
pub struct Cleanup {
    shared: Arc<Shared>,
    worker: Worker,
    next_try: Instant,
}
impl Cleanup {
    pub fn new(work: impl FnOnce() + Send + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                work: Mutex::new(Some(Box::new(work))),
                outcome: Mutex::new(None),
            }),
            worker: Worker::Idle,
            next_try: Instant::now(),
        }
    }
    fn start(&mut self) {
        self.start_with(|work| {
            std::thread::Builder::new()
                .name("blent-retire-work".into())
                .spawn(work)
        });
    }
    fn start_with(
        &mut self,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> std::io::Result<std::thread::JoinHandle<()>>,
    ) {
        if !matches!(self.worker, Worker::Idle) || Instant::now() < self.next_try {
            return;
        }
        self.next_try = Instant::now() + Duration::from_secs(1);
        let shared = self.shared.clone();
        // Spawn rejection leaves the unconsumed work in the admitted record.
        // The worker publishes both outcomes there and returns only unit. Its
        // Rust result packet never owns an application panic payload, so native
        // join ownership can safely transfer without dropping such a payload.
        if let Ok(handle) = spawn(Box::new(move || {
            let outcome = run_work(&shared, || {
                tracing::error!("Resource cleanup panicked; admission remains quarantined");
            });
            // Exactly one worker writes this private, initially empty slot.
            *shared.outcome.lock().unwrap() = Some(outcome);
        })) {
            self.worker = Worker::Running(native::Join::new(handle));
        }
    }
    fn poll_worker(&mut self) -> bool {
        if let Worker::Running(worker) = &mut self.worker {
            if worker.poll_complete() {
                self.worker = Worker::Joined;
            }
        }
        matches!(self.worker, Worker::Joined)
    }
}

fn run_work(shared: &Shared, report: fn()) -> Outcome {
    let work = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let work = shared.work.lock().unwrap().take().unwrap();
        work();
    }));
    let report = if work.is_err() {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(report))
    } else {
        Ok(())
    };
    Outcome { work, report }
}

impl Observe for Cleanup {
    fn poll_complete(&mut self) -> bool {
        self.start();
        if !self.poll_worker() {
            return false;
        }
        let outcome = self.shared.outcome.lock().unwrap();
        outcome
            .as_ref()
            .is_some_and(|outcome| outcome.work.is_ok() && outcome.report.is_ok())
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

#[cfg(test)]
#[path = "retirement_work/tests.rs"]
mod native_lifetime_tests;
