//! T1432: process exit retains admitted control work and original TLS teardown.
use blent::control_workers::{Worker, WorkerPool};
use blent_config::commands::SyncCommandExt;
use std::{cell::RefCell, process::Command, sync::mpsc, time::Duration};

type Gate = (mpsc::Sender<()>, mpsc::Receiver<()>);
struct Held(Gate);
impl Drop for Held {
    fn drop(&mut self) {
        self.0 .0.send(()).unwrap();
        self.0 .1.recv().unwrap();
    }
}
thread_local! { static TLS: RefCell<Option<Held>> = const { RefCell::new(None) }; }

fn native(gate: Gate, tls: bool) {
    let held = Held(gate);
    if tls {
        TLS.with(|slot| *slot.borrow_mut() = Some(held));
    }
}

async fn workflow(worker: Worker, mode: &str) -> Result<(), &'static str> {
    if mode == "cancel" {
        let mut future = Box::pin(async move {
            let _worker = worker;
            std::future::pending::<()>().await;
        });
        assert!(futures_util::poll!(&mut future).is_pending());
        drop(future);
        return Ok(());
    }
    let _worker = worker;
    match mode {
        "panic" => panic!("T1432: unwind with admitted worker"),
        "error" => Err("T1432: workflow error"),
        _ => Ok(()),
    }
}

fn held_exit(mode: String, tls: bool) {
    let pool = WorkerPool::new(1).unwrap();
    let (started, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let (receipt, received) = mpsc::channel();
    let (done, returned) = mpsc::channel();
    let original_pool = pool.clone();
    let original = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(|| {
            blent::runtime_owner::run(async {
                let worker = Worker::spawn(original_pool, "t1432-original", move || {
                    native((started, held), tls);
                })
                .unwrap();
                receipt.send(worker.completion()).unwrap();
                workflow(worker, &mode).await
            })
            .unwrap()
        });
        assert_outcome(outcome, &mode);
        done.send(()).unwrap();
    });
    entered.recv().unwrap();
    let proof = received.recv().unwrap();
    while blent::runtime_owner::check_running().is_ok() {
        std::thread::yield_now();
    }
    let premature = returned.recv_timeout(Duration::from_millis(100)).is_ok();
    assert!(!proof.is_complete());
    assert_eq!(pool.active(), 1);
    let rejected = Worker::spawn(WorkerPool::new(1).unwrap(), "t1432-rejected", || {}).is_err();
    release.send(()).unwrap();
    original.join().unwrap();
    while !proof.is_complete() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(pool.active(), 0);
    assert!(
        !premature,
        "T1432: runtime returned before original control worker/TLS exit"
    );
    assert!(
        rejected,
        "T1432: control admission accepted after process cutoff"
    );
}

fn assert_outcome(outcome: std::thread::Result<Result<(), &'static str>>, mode: &str) {
    match mode {
        "panic" => assert!(outcome.is_err()),
        "error" => assert_eq!(outcome.unwrap(), Err("T1432: workflow error")),
        _ => assert_eq!(outcome.unwrap(), Ok(())),
    }
}

#[test]
fn t1432_control_exit_fixture() {
    if let Ok(mode) = std::env::var("BLENT_T1432_EXIT") {
        held_exit(mode, std::env::var_os("BLENT_T1432_TLS").is_some());
    }
}

#[test]
fn t1432_exit_waits_for_original_control_work_and_tls() {
    for mode in ["normal", "error", "cancel", "panic"] {
        for tls in [false, true] {
            run_child(mode, tls);
        }
    }
}

fn run_child(mode: &str, tls: bool) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "t1432_control_exit_fixture", "--nocapture"])
        .env("BLENT_T1432_EXIT", mode)
        .env("BLENT_RUNTIME_STOP_MS", "20")
        .env_remove("BLENT_T1432_TLS");
    if tls {
        command.env("BLENT_T1432_TLS", "1");
    }
    let output = command.output_timeout(Duration::from_secs(20)).unwrap();
    assert!(
        output.status.success(),
        "T1432 {mode} TLS={tls}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[path = "control_exit/config.rs"]
mod config;

#[test]
fn t1432_configuration_exit_fixture() {
    if let Ok(mode) = std::env::var("BLENT_T1432_CONFIG") {
        config::run(mode);
    }
}

#[test]
fn t1432_shared_configuration_pool_closes_without_static_deadlock() {
    for mode in ["normal", "error", "panic", "tls", "queue", "invalid"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "t1432_configuration_exit_fixture", "--nocapture"])
            .env("BLENT_T1432_CONFIG", mode)
            .env(
                "BLENT_CONFIG_WORKERS",
                if mode == "invalid" { "0" } else { "1" },
            )
            .env("BLENT_RUNTIME_STOP_MS", "20")
            .output_timeout(Duration::from_secs(20))
            .unwrap();
        assert!(
            output.status.success(),
            "T1432 shared config {mode}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
