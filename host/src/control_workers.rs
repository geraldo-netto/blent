//! Bounded control-thread ownership using the shared retirement budget.
pub use blent_config::retirement::Completion;
use blent_config::retirement::{Admission, Observe, Pool};
use std::{io, sync::OnceLock, thread::JoinHandle};
#[cfg(test)]
use std::{sync::mpsc, time::Duration};
#[derive(Clone, Debug)]
pub struct WorkerPool(Pool);
impl WorkerPool {
    pub fn new(capacity: usize) -> io::Result<Self> {
        validate_capacity(capacity)?;
        Ok(Self(Pool::new(capacity)?))
    }
    pub fn active(&self) -> usize {
        self.0.active()
    }
    pub fn shared() -> io::Result<Self> {
        static POOL: OnceLock<io::Result<WorkerPool>> = OnceLock::new();
        POOL.get_or_init(|| {
            let capacity = match std::env::var("BLENT_CONTROL_WORKERS") {
                Ok(value) => parse_capacity(&value)?,
                Err(std::env::VarError::NotPresent) => 8,
                Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidInput, error)),
            };
            Self::new(capacity)
        })
        .as_ref()
        .cloned()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))
    }
}
fn validate_capacity(capacity: usize) -> io::Result<()> {
    if !(1..=64).contains(&capacity) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Control worker capacity must be 1..64",
        ));
    }
    Ok(())
}
fn parse_capacity(value: &str) -> io::Result<usize> {
    let capacity = value
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    validate_capacity(capacity)?;
    Ok(capacity)
}

struct Thread {
    handle: Option<JoinHandle<()>>,
    cleanup: Option<blent_config::retirement_work::Cleanup>,
}
impl Thread {
    fn new(handle: JoinHandle<()>) -> Self {
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
            // is_finished precedes native TLS destruction. Never join on the
            // shared reaper; this cleanup retains the original worker's slot.
            let handle = self.handle.take().unwrap();
            self.cleanup = Some(blent_config::retirement_work::Cleanup::new(move || {
                let _ = handle.join();
            }));
        }
        self.cleanup.as_mut().unwrap().poll_complete()
    }
}

pub struct Worker {
    owned: Option<(Thread, Admission)>,
}
impl Worker {
    pub fn spawn(
        pool: WorkerPool,
        name: &str,
        run: impl FnOnce() + Send + 'static,
    ) -> io::Result<Self> {
        let slot = pool.0.reserve()?;
        let handle = std::thread::Builder::new().name(name.into()).spawn(run)?;
        Ok(Self {
            owned: Some((Thread::new(handle), slot)),
        })
    }
    pub fn completion(&self) -> Completion {
        self.owned.as_ref().unwrap().1.completion()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some((thread, slot)) = self.owned.take() {
            slot.retain(thread);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn completed(ticket: &Completion) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !ticket.is_complete() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn t811_held_retirement_keeps_capacity_until_join_and_never_blocks_other_completions() {
        let pool = WorkerPool::new(2).unwrap();
        let (release, hold) = mpsc::channel();
        let first = Worker::spawn(pool.clone(), "held", move || {
            hold.recv().unwrap();
        })
        .unwrap();
        let ticket = first.completion();
        drop(first);
        let second = Worker::spawn(pool.clone(), "quick", || {}).unwrap();
        let ready = second.completion();
        assert_eq!(pool.active(), 2);
        assert!(Worker::spawn(pool.clone(), "excess", || {}).is_err());
        drop(second);
        completed(&ready);
        assert_eq!(pool.active(), 1);
        assert!(!ticket.is_complete());
        release.send(()).unwrap();
        completed(&ticket);
        assert_eq!(pool.active(), 0);
        let failed = Worker::spawn(pool.clone(), "panic", || panic!("fixture panic")).unwrap();
        let failed_ticket = failed.completion();
        drop(failed);
        completed(&failed_ticket);
        assert_eq!(pool.active(), 0);
    }
    #[test]
    fn t814_thread_local_cleanup_cannot_block_other_retirements() {
        use std::cell::RefCell;
        struct Held {
            entered: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
        }
        impl Drop for Held {
            fn drop(&mut self) {
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
            }
        }
        thread_local! { static HELD: RefCell<Option<Held>> = const { RefCell::new(None) }; }
        let pool = WorkerPool::new(2).unwrap();
        let (started, entered) = mpsc::channel();
        let (release, hold) = mpsc::channel();
        let worker = Worker::spawn(pool.clone(), "held-tls", move || {
            HELD.with(|slot| {
                *slot.borrow_mut() = Some(Held {
                    entered: started,
                    release: hold,
                })
            });
        })
        .unwrap();
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let first = worker.completion();
        drop(worker);
        let sibling = Worker::spawn(pool.clone(), "quick-sibling", || {}).unwrap();
        let second = sibling.completion();
        drop(sibling);
        let end = std::time::Instant::now() + Duration::from_millis(150);
        while !second.is_complete() && std::time::Instant::now() < end {
            std::thread::sleep(Duration::from_millis(1));
        }
        let sibling_completed = second.is_complete();
        assert!(!first.is_complete());
        release.send(()).unwrap();
        completed(&first);
        completed(&second);
        assert!(
            sibling_completed,
            "T814: joining a worker with held TLS cleanup stalled the shared reaper"
        );
    }
    #[test]
    fn t811_capacity_rejects_invalid_values_and_preserves_bounded_admission() {
        for value in [
            "",
            "-1",
            "0",
            "65",
            "9999999999999999999999999999",
            "NaN",
            "1.5",
            " 8",
            "8 ",
        ] {
            assert!(parse_capacity(value).is_err(), "{value}");
        }
        for value in 1..=64 {
            assert_eq!(parse_capacity(&value.to_string()).unwrap(), value);
        }
        for value in [0, 65, usize::MAX] {
            assert!(WorkerPool::new(value).is_err());
        }
        assert_eq!(
            WorkerPool::shared().unwrap().0.capacity(),
            WorkerPool::shared().unwrap().0.capacity()
        );
    }
}
