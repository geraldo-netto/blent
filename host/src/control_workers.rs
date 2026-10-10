//! Bounded ownership for host control threads. Dropping an owner never joins on
//! the caller; one reaper retains each thread until its real termination.
use std::io;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc, OnceLock,
};
use std::thread::JoinHandle;
use std::time::Duration;

/// Retirement observation contains no thread/native handle. Completion means
/// the worker has terminated and has been joined, not that a timer expired.
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
    retired: Sender<Retiring>,
}

/// Capacity includes active and retiring control workers. Share the same pool
/// across replacement owners; a held retirement must retain its admission slot.
#[derive(Clone, Debug)]
pub struct WorkerPool(Arc<State>);
impl WorkerPool {
    pub fn new(capacity: usize) -> io::Result<Self> {
        validate_capacity(capacity)?;
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("blent-control-reaper".into())
            .spawn(move || reap(receiver))?;
        Ok(Self(Arc::new(State {
            capacity,
            used: AtomicUsize::new(0),
            retired: sender,
        })))
    }

    pub fn active(&self) -> usize {
        self.0.used.load(Ordering::Acquire)
    }

    pub(crate) fn shared() -> io::Result<Self> {
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

    fn reserve(&self) -> io::Result<Reservation> {
        self.0
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < self.0.capacity).then_some(used + 1)
            })
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Control workers still active or retiring",
                )
            })?;
        Ok(Reservation(self.clone()))
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

#[derive(Debug)]
struct Reservation(WorkerPool);
impl Drop for Reservation {
    fn drop(&mut self) {
        self.0 .0.used.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Debug)]
struct Retiring {
    handle: JoinHandle<()>,
    complete: Completion,
    _reservation: Reservation,
}

pub(crate) struct Worker {
    owned: Option<Retiring>,
    sender: Sender<Retiring>,
}
impl Worker {
    pub(crate) fn spawn(
        pool: WorkerPool,
        name: &str,
        run: impl FnOnce() + Send + 'static,
    ) -> io::Result<Self> {
        let reservation = pool.reserve()?;
        let handle = std::thread::Builder::new().name(name.into()).spawn(run)?;
        Ok(Self {
            owned: Some(Retiring {
                handle,
                complete: Completion(Arc::new(AtomicBool::new(false))),
                _reservation: reservation,
            }),
            sender: pool.0.retired.clone(),
        })
    }
    pub(crate) fn completion(&self) -> Completion {
        self.owned.as_ref().unwrap().complete.clone()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(owned) = self.owned.take() {
            // At most capacity records exist: each record owns a reservation
            // acquired before its thread starts. Sending cannot block.
            self.sender
                .send(owned)
                .expect("Control reaper outlives its reservations");
        }
    }
}

fn reap(receiver: Receiver<Retiring>) {
    let mut pending = Vec::new();
    loop {
        let message = if pending.is_empty() {
            receiver
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        } else {
            receiver.recv_timeout(Duration::from_millis(10))
        };
        match message {
            Ok(owned) => pending.push(owned),
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        reap_finished(&mut pending);
    }
}
fn reap_finished(pending: &mut Vec<Retiring>) {
    for index in (0..pending.len()).rev() {
        if pending[index].handle.is_finished() {
            let owned = pending.swap_remove(index);
            let _ = owned.handle.join();
            drop(owned._reservation);
            owned.complete.0.store(true, Ordering::Release);
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
            WorkerPool::shared().unwrap().0.capacity,
            WorkerPool::shared().unwrap().0.capacity
        );
    }
}
