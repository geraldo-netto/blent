//! T1435: headless original media controller/native and TLS retirement.
use super::*;
use blent::{camera_control::Controller, control_workers::WorkerPool};
use blent_config::{camera::CameraBackend, retirement::Completion};
use std::{
    cell::RefCell,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

type Gate = (mpsc::Sender<()>, mpsc::Receiver<()>);
struct Held(Gate);
impl Drop for Held {
    fn drop(&mut self) {
        self.0 .0.send(()).unwrap();
        self.0 .1.recv().unwrap();
    }
}
thread_local! { static TLS: RefCell<Option<Held>> = const { RefCell::new(None) }; }

fn controller(pool: WorkerPool, gate: Gate, tls: bool) -> Completion {
    let (started, entered) = mpsc::channel();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let camera = Controller::with_pool(pool, move |_, _, _| {
        let held = Held(gate.lock().unwrap().take().unwrap());
        let started = started.clone();
        async move {
            started.send(()).unwrap();
            if tls {
                TLS.with(|slot| *slot.borrow_mut() = Some(held));
            } else {
                drop(held);
            }
            Ok(())
        }
    })
    .unwrap();
    camera
        .start(blent_config::camera::CameraProfile {
            serial: Some("T1435-private-camera".into()),
            ..Default::default()
        })
        .unwrap();
    entered.recv().unwrap();
    camera.shutdown()
}

fn outcome(mode: &str) -> eframe::Result {
    match mode {
        "panic" => panic!("T1435 original unwind"),
        "error" => Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            "T1435 original error",
        )))),
        _ => Ok(()),
    }
}

fn assert_outcome(result: std::thread::Result<eframe::Result>, mode: &str) {
    match mode {
        "panic" => assert_eq!(
            result.unwrap_err().downcast_ref::<&str>(),
            Some(&"T1435 original unwind")
        ),
        "error" => assert!(result
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("T1435 original error")),
        _ => assert!(result.unwrap().is_ok()),
    }
}

fn held_exit(mode: String, tls: bool) {
    let pool = WorkerPool::new(1).unwrap();
    let original_pool = pool.clone();
    let (started, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let (receipt, proof) = mpsc::channel();
    let (done, returned) = mpsc::channel();
    let original = std::thread::spawn(move || {
        let caller = std::thread::current().id();
        let result = std::panic::catch_unwind(|| {
            run_gui(|| {
                assert_eq!(std::thread::current().id(), caller);
                receipt
                    .send(controller(original_pool, (started, held), tls))
                    .unwrap();
                outcome(&mode)
            })
        });
        assert_outcome(result, &mode);
        done.send(()).unwrap();
    });
    entered.recv().unwrap();
    let proof = proof.recv().unwrap();
    let early = returned.recv_timeout(Duration::from_millis(100)).is_ok();
    let pending = !proof.is_complete();
    let active = pool.active();
    release.send(()).unwrap();
    original.join().unwrap();
    while !proof.is_complete() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(pending);
    assert_eq!(active, 1);
    assert_eq!(pool.active(), 0);
    assert!(
        !early,
        "T1435 GUI entry returned before original camera/control/TLS retirement"
    );
}

#[test]
fn t1435_gui_exit_fixture() {
    if let Ok(mode) = std::env::var("BLENT_T1435_EXIT") {
        if mode == "capacity" {
            assert!(run_gui(|| panic!("T1435 invalid runtime must not start GUI")).is_err());
            return;
        }
        held_exit(mode, std::env::var_os("BLENT_T1435_TLS").is_some());
    }
}

#[test]
fn t1435_gui_entry_retains_camera_and_original_control_thread() {
    for mode in ["normal", "error", "panic"] {
        for tls in [false, true] {
            run_child(mode, tls);
        }
    }
    run_child("capacity", false);
}

fn run_child(mode: &str, tls: bool) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "platform::lifetime::tests::t1435_gui_exit_fixture",
            "--nocapture",
        ])
        .env("BLENT_T1435_EXIT", mode)
        .env(
            "BLENT_RUNTIME_STOP_MS",
            if mode == "capacity" { "0" } else { "20" },
        )
        .env_remove("BLENT_T1435_TLS");
    if tls {
        command.env("BLENT_T1435_TLS", "1");
    }
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{mode} TLS={tls}: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
