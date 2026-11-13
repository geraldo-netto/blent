//! T1422: original uinput constructors remain owned after follower cancellation.
#![cfg(target_os = "linux")]
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::watch;

const TEST: &str = "t1422_cancelled_creation_retains_native_proof_and_identity";

fn fixture(test: &str, failure: u64, input_limit: &str) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let library = root.join("uinput.so");
    let compiled = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-Wall", "-Wextra", "-Werror"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/uinput_mock.c"))
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    std::fs::write(root.join("device"), "").unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("LD_PRELOAD", &library)
        .env("BLENT_INPUT_LIMIT", input_limit)
        .env("BLENT_T497_UINPUT_FILE", root.join("device"))
        .env("BLENT_T497_UINPUT_LOG", root.join("ioctl"))
        .env("BLENT_T497_UINPUT_FAIL", failure.to_string())
        .env("BLENT_T1422_GATE", root.join("release"))
        .env("PATH", root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root)
        .env("XDG_RUNTIME_DIR", root)
        .env("XDG_CURRENT_DESKTOP", "test")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn intercepted() {
    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"blent_test_uinput_active".as_ptr()) };
    assert!(!symbol.is_null(), "T1422 refuses real device access");
    let probe: unsafe extern "C" fn() -> i32 = unsafe { std::mem::transmute(symbol) };
    assert_eq!(unsafe { probe() }, 497);
}
struct Release(PathBuf);
impl Drop for Release {
    fn drop(&mut self) {
        std::fs::write(&self.0, "release").unwrap();
    }
}
fn counts() -> (usize, usize) {
    let log = std::fs::read_to_string(std::env::var_os("BLENT_T497_UINPUT_LOG").unwrap())
        .unwrap_or_default();
    (
        log.matches("request 21761\n").count(),
        log.matches("request 21762\n").count(),
    )
}
async fn created() {
    wait_creates(1).await;
}
async fn wait_creates(expected: usize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while counts().0 != expected {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn complete(proof: &blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn cancellation() {
    let release = Release(std::env::var_os("BLENT_T1422_GATE").unwrap().into());
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let config = InputConfig {
        touch: true,
        pen: false,
        pointer: false,
        ..Default::default()
    };
    let backend = LinuxBackend::new(card.clone()).unwrap();
    let proof = backend.native_completion();
    let first = tokio::spawn(backend.follow(tablet.clone(), mode.clone(), config.clone()));
    created().await;
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert!(
        proof.is_some(),
        "T1422: admitted constructor has no native completion"
    );
    let proof = proof.unwrap();
    assert!(!proof.is_complete());
    let replacement = tokio::spawn(backend.follow(tablet.clone(), mode.clone(), config.clone()));
    replacement.await.unwrap();
    assert_eq!(
        counts(),
        (1, 0),
        "T1422: original identity reused during creation"
    );
    let contender = LinuxBackend::new(card.clone()).unwrap();
    contender
        .follow(tablet.clone(), mode.clone(), config.clone())
        .await;
    assert_eq!(
        counts(),
        (1, 0),
        "T1422: another backend reused the identity"
    );
    drop(contender);
    drop(backend);
    assert!(!proof.is_complete());
    drop(release);
    complete(&proof).await;
    assert_eq!(counts(), (1, 1));
    let replacement = LinuxBackend::new(card).unwrap();
    let proof = replacement.native_completion().unwrap();
    let task = tokio::spawn(replacement.follow(tablet, mode, config));
    wait_creates(2).await;
    task.abort();
    let _ = task.await;
    drop(replacement);
    complete(&proof).await;
    assert_eq!(counts(), (2, 2), "T1422: settled identity must be reusable");
}
#[test]
fn t1422_cancelled_creation_retains_native_proof_and_identity() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(TEST, 0, "64");
        return;
    }
    intercepted();
    blent::runtime_owner::run(async {
        cancellation().await;
    })
    .unwrap();
}

async fn abandoned(started: std::sync::mpsc::Sender<blent_config::retirement::Completion>) {
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card).unwrap();
    let proof = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(
        tablet,
        mode,
        InputConfig {
            touch: true,
            pen: false,
            pointer: false,
            ..Default::default()
        },
    ));
    created().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    started.send(proof).unwrap();
}
struct ExitWorker {
    release: Option<Release>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ExitWorker {
    fn settle(&mut self) -> std::thread::Result<()> {
        drop(self.release.take());
        self.thread.take().unwrap().join()
    }
}
impl Drop for ExitWorker {
    fn drop(&mut self) {
        drop(self.release.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn process_exit() {
    let release = Release(std::env::var_os("BLENT_T1422_GATE").unwrap().into());
    let (started, original) = std::sync::mpsc::channel();
    let (finished, returned) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        blent::runtime_owner::run(abandoned(started)).unwrap();
        finished.send(()).unwrap();
    });
    let mut worker = ExitWorker {
        release: Some(release),
        thread: Some(worker),
    };
    let proof = original.recv_timeout(Duration::from_secs(3)).unwrap();
    let early = returned.recv_timeout(Duration::from_millis(1200));
    let unsettled = !proof.is_complete();
    worker.settle().unwrap();
    assert!(
        early.is_err(),
        "T1422: process exit abandoned the original constructor"
    );
    assert!(unsettled);
    assert!(proof.is_complete());
    assert_eq!(counts(), (1, 1));
}
#[test]
fn t1422_process_exit_waits_for_cancelled_constructor() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(
            "t1422_process_exit_waits_for_cancelled_constructor",
            0,
            "64",
        );
        return;
    }
    intercepted();
    process_exit();
}

async fn failed_creation() {
    let release = Release(std::env::var_os("BLENT_T1422_GATE").unwrap().into());
    drop(release);
    let (_card, card) = watch::channel(None);
    let (tablet, attached) = watch::channel(true);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card).unwrap();
    let proof = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(
        attached,
        mode,
        InputConfig {
            touch: true,
            pen: false,
            pointer: false,
            ..Default::default()
        },
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    drop(tablet);
    task.await.unwrap();
    drop(backend);
    complete(&proof).await;
    assert_eq!(counts(), (0, 0));
}
#[test]
fn t1422_failed_constructor_settles_original_admission() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(
            "t1422_failed_constructor_settles_original_admission",
            0x5501,
            "64",
        );
        return;
    }
    intercepted();
    blent::runtime_owner::run(failed_creation()).unwrap();
}

struct Capture;
impl blent::capture_contract::CaptureBackend for Capture {
    fn start(
        self: Box<Self>,
        mut context: blent::capture_contract::CaptureContext,
    ) -> blent::capture_contract::CaptureWorkers {
        blent::capture_contract::CaptureWorkers {
            capture: Box::pin(async move {
                let _ = context.stop.wait_for(|stop| *stop).await;
                Ok(())
            }),
            auxiliary: vec![],
        }
    }
}
fn session_spec() -> blent::session::Spec {
    blent::session::Spec {
        instance: 0,
        ports: (0, 0),
        token: None,
        devices: (true, false, false),
        settings: blent::media::EncoderSettings {
            encoder: "libx264".into(),
            fps: 30,
            bitrate: 20000,
            width: 640,
            height: 480,
            quality: 18,
            width_mm: 310,
            height_mm: 194,
            stream_scale: 1,
            geometry_ready: true,
            decoders: None,
            decoder_epoch: 0,
            selection: None,
        },
    }
}
async fn session_retirement() {
    let release = Release(std::env::var_os("BLENT_T1422_GATE").unwrap().into());
    let (_card, card) = watch::channel(None);
    let backend = std::sync::Arc::new(LinuxBackend::new(card).unwrap());
    let native = backend.native_completion().unwrap();
    let prepared = session_spec().prepare(
        watch::channel(false).0,
        Default::default(),
        Box::new(Capture),
        backend,
    );
    prepared.tablet.send(true).unwrap();
    let (_shutdown, shutdown) = watch::channel(false);
    let runtime = prepared.start(shutdown).await.unwrap();
    created().await;
    let proof = runtime.stop().await;
    assert!(
        !proof.is_complete(),
        "T1422: session retired before its original constructor"
    );
    assert!(!native.is_complete());
    assert_eq!(counts(), (1, 0));
    drop(release);
    complete(&proof).await;
    assert!(native.is_complete());
    assert_eq!(counts(), (1, 1));
}
#[test]
fn t1422_session_retirement_waits_for_original_constructor() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(
            "t1422_session_retirement_waits_for_original_constructor",
            0,
            "64",
        );
        return;
    }
    intercepted();
    blent::runtime_owner::run(session_retirement()).unwrap();
}

async fn capacity() {
    let (_card, card) = watch::channel(None);
    let backend = LinuxBackend::new(card.clone()).unwrap();
    let proof = backend.native_completion().unwrap();
    assert!(LinuxBackend::new(card.clone()).is_err());
    drop(backend);
    complete(&proof).await;
    let replacement = LinuxBackend::new(card).unwrap();
    let proof = replacement.native_completion().unwrap();
    drop(replacement);
    complete(&proof).await;
    assert_eq!(counts(), (0, 0));
}
#[test]
fn t1422_capacity_rejects_before_creation_and_recovers_after_settlement() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(
            "t1422_capacity_rejects_before_creation_and_recovers_after_settlement",
            0,
            "1",
        );
        return;
    }
    intercepted();
    blent::runtime_owner::run(capacity()).unwrap();
    assert!(
        LinuxBackend::new(watch::channel(None).1).is_err(),
        "T1422: process cutoff must reject admission"
    );
}
#[test]
fn t1422_invalid_capacity_never_admits_native_work() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        for invalid in ["0", "4097", "invalid"] {
            fixture(
                "t1422_invalid_capacity_never_admits_native_work",
                0,
                invalid,
            );
        }
        return;
    }
    intercepted();
    assert!(LinuxBackend::new(watch::channel(None).1).is_err());
    assert_eq!(counts(), (0, 0));
}

async fn identity_boundary(instance: u32, valid: bool) {
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(false);
    let (_mode, mode) = watch::channel(false);
    let backend = LinuxBackend::new(card).unwrap();
    let proof = backend.native_completion().unwrap();
    let config = InputConfig {
        instance,
        touch: false,
        pen: false,
        pointer: false,
        ..Default::default()
    };
    let follow = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        backend.follow(tablet.clone(), mode.clone(), config.clone())
    }));
    assert!(
        follow.is_ok(),
        "T1430 invalid identity must reject without panic"
    );
    let mut follow = follow.unwrap();
    assert_eq!(futures_util::poll!(&mut follow).is_pending(), valid);
    drop(follow);
    if !valid {
        let mut current = backend.follow(
            tablet,
            mode,
            InputConfig {
                instance: 0,
                ..config
            },
        );
        assert!(
            futures_util::poll!(&mut current).is_pending(),
            "T1430 rejected identity consumed the follower"
        );
        drop(current);
    }
    drop(backend);
    complete(&proof).await;
    assert_eq!(counts(), (0, 0));
}

#[test]
fn t1430_identity_boundary_rejects_without_panic_or_admission() {
    if std::env::var_os("BLENT_T1422_GATE").is_none() {
        fixture(
            "t1430_identity_boundary_rejects_without_panic_or_admission",
            0,
            "64",
        );
        return;
    }
    intercepted();
    blent::runtime_owner::run(async {
        for instance in [0, 4095] {
            identity_boundary(instance, true).await;
        }
        for instance in [4096, 65535, 65536, u32::MAX] {
            identity_boundary(instance, false).await;
        }
    })
    .unwrap();
}
