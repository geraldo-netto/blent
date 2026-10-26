use super::*;
use crate::retirement::{Completion, Pool};
use std::{
    cell::RefCell,
    sync::{atomic::Ordering, mpsc},
};

struct Tail {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl Drop for Tail {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.recv();
    }
}
thread_local! { static TAIL: RefCell<Option<Tail>> = const { RefCell::new(None) }; }

fn complete(proof: &Completion) {
    let end = Instant::now() + Duration::from_secs(5);
    while !proof.is_complete() {
        assert!(Instant::now() < end, "T1260 exact worker did not retire");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn held_tail() -> (Tail, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered, observed) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    (
        Tail {
            entered,
            release: blocked,
        },
        observed,
        release,
    )
}

#[test]
fn t1260_cleanup_tls_keeps_capacity_while_siblings_retire() {
    let pool = Pool::new(2).unwrap();
    let slot = pool.reserve().unwrap();
    let proof = slot.completion();
    let (tail, entered, release) = held_tail();
    slot.retain(Cleanup::new(move || {
        TAIL.with(|slot| *slot.borrow_mut() = Some(tail))
    }));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let sibling = pool.reserve().unwrap();
    let sibling_proof = sibling.completion();
    sibling.retain(Cleanup::new(|| {}));
    complete(&sibling_proof);
    let reused = pool.reserve().unwrap();
    let prematurely_released = proof.is_complete();
    let over_capacity = pool.reserve().is_ok();
    release.send(()).unwrap();
    complete(&proof);
    drop(reused);
    assert!(
        !prematurely_released,
        "T1260 cleanup reported completion before TLS exit"
    );
    assert!(
        !over_capacity,
        "T1260 original native thread lost its admission"
    );
    assert_eq!(pool.active(), 0);
}

#[test]
fn t1260_thread_tls_join_is_nonblocking_and_repeatable() {
    let (tail, entered, release) = held_tail();
    let handle = std::thread::spawn(move || TAIL.with(|slot| *slot.borrow_mut() = Some(tail)));
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut worker = Thread::new(handle);
    let (reported, report) = mpsc::channel();
    let poller = std::thread::spawn(move || {
        reported.send(worker.poll_complete()).unwrap();
        worker
    });
    let before_release = report.recv_timeout(Duration::from_secs(5));
    release.send(()).unwrap();
    let mut worker = poller.join().unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while !worker.poll_complete() {
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    assert!(worker.poll_complete());
    assert!(!before_release.unwrap(), "T1260 poll blocked on native TLS");
}

#[test]
fn t1260_spawn_rejection_preserves_work_and_backoff() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    let mut cleanup = Cleanup::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    cleanup.start_with(|_| Err(std::io::Error::other("T1260 rejected spawn")));
    assert!(matches!(cleanup.worker, Worker::Idle));
    assert!(cleanup.shared.work.lock().unwrap().is_some());
    cleanup.start_with(|_| panic!("T1260 retry ignored backoff"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    cleanup.next_try = Instant::now();
    let end = Instant::now() + Duration::from_secs(5);
    while !cleanup.poll_complete() {
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    cleanup.start_with(|_| panic!("T1260 completed cleanup spawned twice"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn t1260_panicked_native_worker_still_retires() {
    let mut worker = Thread::new(std::thread::spawn(|| panic!("T1260 native worker")));
    let end = Instant::now() + Duration::from_secs(5);
    while !worker.poll_complete() {
        assert!(Instant::now() < end);
        std::thread::yield_now();
    }
    assert!(worker.poll_complete());
}

#[test]
fn t1260_unsubmitted_owner_drop_has_owned_process_boundary() {
    if std::env::var_os("BLENT_T1260_DROP_CHILD").is_some() {
        drop(Thread::new(std::thread::spawn(|| {})));
        drop(native::Join::new(std::thread::spawn(|| {})));
        return;
    }
    // Dropping an unsubmitted native join right is a supported adapter boundary.
    // The parent owns/reaps this entire child, including any detached exit tail.
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "retirement_work::native_lifetime_tests::t1260_unsubmitted_owner_drop_has_owned_process_boundary"])
        .env("BLENT_T1260_DROP_CHILD", "1")
        .status().unwrap();
    assert!(status.success());
}

struct PanickingPayload(mpsc::Sender<()>);
impl Drop for PanickingPayload {
    fn drop(&mut self) {
        let _ = self.0.send(());
        panic!("T1260 panic payload destructor");
    }
}

#[test]
fn t1260_panic_payload_drop_cannot_abort_retirement_owner() {
    if std::env::var_os("BLENT_T1260_PAYLOAD_CHILD").is_some() {
        let pool = Pool::new(2).unwrap();
        let slot = pool.reserve().unwrap();
        let original = slot.completion();
        let (entered, observed) = mpsc::channel();
        slot.retain(Thread::new(std::thread::spawn(move || {
            std::panic::panic_any(PanickingPayload(entered));
        })));
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        let sibling = pool.reserve().unwrap();
        let retired = sibling.completion();
        sibling.retain(Cleanup::new(|| {}));
        complete(&retired);
        assert!(!original.is_complete());
        assert_eq!(pool.active(), 1);
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "retirement_work::native_lifetime_tests::t1260_panic_payload_drop_cannot_abort_retirement_owner"])
        .env("BLENT_T1260_PAYLOAD_CHILD", "1")
        .status().unwrap();
    assert!(
        status.success(),
        "T1260 native join lost Rust panic-payload ownership"
    );
}

#[test]
fn t1260_work_and_reporting_panics_remain_separately_owned() {
    let shared = Shared {
        work: Mutex::new(Some(Box::new(|| panic!("T1260 work")))),
        outcome: Mutex::new(None),
    };
    // This pure boundary owns two string payloads; no native resources escape.
    let outcome = run_work(&shared, || panic!("T1260 reporting"));
    assert!(outcome.work.is_err());
    assert!(outcome.report.is_err());
}
