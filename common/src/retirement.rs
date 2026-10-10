//! Bounded retirement ownership, independent of process/thread/task adapters.
//! Reserve before starting a resource. Transfer its admission together with its
//! owner; release capacity only after observation and destruction both complete.
use std::{
    io,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

/// Implementations must poll without blocking. True confirms actual completion,
/// including any required join/reap. A deadline alone must never return true.
pub trait Observe: Send + 'static {
    fn poll_complete(&mut self) -> bool;
}
#[derive(Clone, Debug)]
pub struct Completion(Arc<AtomicBool>);
impl Completion {
    pub fn is_complete(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
#[derive(Debug)]
struct State {
    capacity: usize,
    used: AtomicUsize,
    retired: mpsc::Sender<Record>,
}
#[derive(Clone, Debug)]
pub struct Pool(Arc<State>);
impl Pool {
    pub fn new(capacity: usize) -> io::Result<Self> {
        if !(1..=4096).contains(&capacity) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Resource capacity must be 1..4096",
            ));
        }
        let (retired, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("blent-retirement".into())
            .spawn(move || reap(receiver))?;
        Ok(Self(Arc::new(State {
            capacity,
            used: AtomicUsize::new(0),
            retired,
        })))
    }
    pub fn capacity(&self) -> usize {
        self.0.capacity
    }
    pub fn active(&self) -> usize {
        self.0.used.load(Ordering::Acquire)
    }
    pub fn reserve(&self) -> io::Result<Admission> {
        self.0
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < self.0.capacity).then_some(used + 1)
            })
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Resources still active or retiring",
                )
            })?;
        Ok(Admission {
            pool: self.clone(),
            complete: Completion(Arc::new(AtomicBool::new(false))),
        })
    }
}
#[derive(Debug)]
pub struct Admission {
    pool: Pool,
    complete: Completion,
}
impl Admission {
    pub fn completion(&self) -> Completion {
        self.complete.clone()
    }
    pub fn retain(self, resource: impl Observe) {
        let sender = self.pool.0.retired.clone();
        // At most capacity records exist, because each owns a prior admission.
        assert!(
            sender
                .send(Record {
                    resource: Some(Box::new(resource)),
                    _admission: self,
                    poisoned: false
                })
                .is_ok(),
            "Retirement owner outlives admissions"
        );
    }
}
impl Drop for Admission {
    fn drop(&mut self) {
        self.pool.0.used.fetch_sub(1, Ordering::AcqRel);
        self.complete.0.store(true, Ordering::Release);
    }
}
struct Record {
    resource: Option<Box<dyn Observe>>,
    _admission: Admission,
    poisoned: bool,
}
impl Record {
    fn poll(&mut self) -> bool {
        if self.poisoned {
            return false;
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if !self.resource.as_mut().unwrap().poll_complete() {
                return false;
            }
            drop(self.resource.take());
            true
        }));
        match outcome {
            Ok(complete) => complete,
            Err(_) => {
                self.poisoned = true;
                tracing::error!("Retirement adapter panicked; capacity remains quarantined");
                false
            }
        }
    }
}
fn reap(receiver: mpsc::Receiver<Record>) {
    let mut pending = Vec::new();
    loop {
        let message = if pending.is_empty() {
            receiver
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        } else {
            receiver.recv_timeout(Duration::from_millis(5))
        };
        match message {
            Ok(record) => pending.push(record),
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        reap_ready(&mut pending);
    }
}
fn reap_ready(pending: &mut Vec<Record>) {
    for index in (0..pending.len()).rev() {
        if pending[index].poll() {
            pending.swap_remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Held(Arc<AtomicBool>);
    impl Observe for Held {
        fn poll_complete(&mut self) -> bool {
            self.0.load(Ordering::Acquire)
        }
    }
    fn complete(ticket: &Completion) {
        let end = std::time::Instant::now() + Duration::from_secs(2);
        while !ticket.is_complete() {
            assert!(std::time::Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn t814_retirement_retains_capacity_and_siblings_progress() {
        let pool = Pool::new(2).unwrap();
        let held = Arc::new(AtomicBool::new(false));
        let slot = pool.reserve().unwrap();
        let first = slot.completion();
        slot.retain(Held(held.clone()));
        let slot = pool.reserve().unwrap();
        let second = slot.completion();
        assert!(pool.reserve().is_err());
        assert_eq!(pool.active(), 2);
        slot.retain(Held(Arc::new(AtomicBool::new(true))));
        complete(&second);
        assert_eq!(pool.active(), 1);
        assert!(!first.is_complete());
        held.store(true, Ordering::Release);
        complete(&first);
        assert_eq!(pool.active(), 0);
        let slot = pool.reserve().unwrap();
        let unused = slot.completion();
        drop(slot);
        complete(&unused);
    }
    #[test]
    fn t814_capacity_boundaries_reject_invalid_values() {
        for n in [0, 4097, usize::MAX] {
            assert!(Pool::new(n).is_err());
        }
        for n in [1, 2, 64, 128, 4096] {
            assert_eq!(Pool::new(n).unwrap().active(), 0);
        }
    }
    struct Broken;
    impl Observe for Broken {
        fn poll_complete(&mut self) -> bool {
            panic!("T814 observation failure");
        }
    }
    #[test]
    fn t814_panicking_adapter_keeps_its_slot_without_stalling_siblings() {
        let pool = Pool::new(2).unwrap();
        let slot = pool.reserve().unwrap();
        let bad = slot.completion();
        slot.retain(Broken);
        let slot = pool.reserve().unwrap();
        let good = slot.completion();
        slot.retain(Held(Arc::new(AtomicBool::new(true))));
        complete(&good);
        assert!(!bad.is_complete());
        assert_eq!(pool.active(), 1);
    }
}
