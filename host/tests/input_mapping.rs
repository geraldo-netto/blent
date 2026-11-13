//! T1439: mapping commands cannot outlive their original native-device payload.
#![cfg(target_os = "linux")]
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{path::PathBuf, time::Duration};
use tokio::sync::watch;
#[path = "input_mapping/fixture.rs"]
mod fixture;

async fn pause() {
    tokio::time::sleep(Duration::from_millis(1)).await;
}
async fn started() {
    while std::fs::read_to_string(fixture::child())
        .unwrap_or_default()
        .is_empty()
    {
        pause().await;
    }
}
async fn complete(proof: &blent_config::retirement::Completion) {
    while !proof.is_complete() {
        pause().await;
    }
}
async fn destroyed(count: usize) {
    while fixture::counts().1 < count {
        pause().await;
    }
}
async fn reaped() {
    let pid = std::fs::read_to_string(fixture::child()).unwrap();
    while PathBuf::from(format!("/proc/{pid}")).exists() {
        pause().await;
    }
}
fn config() -> InputConfig {
    InputConfig {
        touch: true,
        pen: false,
        pointer: false,
        ..Default::default()
    }
}

async fn cancellation() {
    let release = fixture::Release::new();
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(true);
    let backend = LinuxBackend::new(card.clone()).unwrap();
    let native = backend.native_completion().unwrap();
    let sink = backend.sink();
    let task = tokio::spawn(backend.follow(tablet, mode.clone(), config()));
    started().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    let device = PathBuf::from(std::env::var_os("BLENT_T497_UINPUT_FILE").unwrap());
    let bytes = std::fs::metadata(&device).unwrap().len();
    sink.touch((0.2, 0.3, 0.4), 0, 0);
    let no_late_input = std::fs::metadata(&device).unwrap().len() == bytes;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let early = (fixture::counts(), native.is_complete());
    let restore_early = fixture::calls().contains("mode i 1");
    let contender = LinuxBackend::new(card).unwrap();
    let contender_native = contender.native_completion().unwrap();
    let (_absent, absent) = watch::channel(false);
    let mut replacement = contender.follow(absent, mode, config());
    let rejected = futures_util::poll!(&mut replacement).is_ready();
    drop((replacement, contender));
    complete(&contender_native).await;
    drop(release);
    complete(&native).await;
    reaped().await;
    println!(
        "held_counts={:?} input_complete={} restored_early={} identity_rejected={rejected}",
        early.0, early.1, restore_early
    );
    assert_eq!(
        early.0,
        (1, 0),
        "T1439 destroyed original while mapper held"
    );
    assert!(!early.1, "T1439 input proof ignored original mapper");
    assert!(
        !restore_early,
        "T1439 restored keyboard before original device destruction"
    );
    assert!(rejected, "T1439 reused original input identity");
    assert!(no_late_input, "retired payload remained live in sink");
    assert_eq!(fixture::counts(), (1, 1));
    assert!(fixture::calls().contains("mode i 1"));
}

#[test]
fn t1439_cancelled_mapping_retains_device_identity_and_osk_order() {
    let name = "t1439_cancelled_mapping_retains_device_identity_and_osk_order";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(cancellation());
}

async fn retained_facade() {
    let release = fixture::Release::new();
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(true);
    let backend = LinuxBackend::new(card).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablet, mode, config()));
    started().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let early = fixture::counts();
    drop(release);
    destroyed(1).await;
    reaped().await;
    drop(backend);
    complete(&native).await;
    assert_eq!(
        early,
        (1, 0),
        "T1439 original destruction preceded mapper settlement"
    );
}

#[test]
fn t1439_cancelled_mapping_cleanup_progresses_with_backend_facade_retained() {
    let name = "t1439_cancelled_mapping_cleanup_progresses_with_backend_facade_retained";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(retained_facade());
}

async fn transition(detach: bool) {
    let release = fixture::Release::new();
    let (card, cards) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(cards).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablets, modes, config()));
    started().await;
    let before = fixture::calls();
    if detach {
        tablet.send(false).unwrap();
    } else {
        card.send(Some(1)).unwrap();
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    if detach {
        for wanted in [true, false, true] {
            tablet.send(wanted).unwrap();
        }
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let held_counts = fixture::counts();
    let held_calls = fixture::calls();
    drop(release);
    while fixture::calls().matches("xinput map-to-output").count() < 2 {
        pause().await;
    }
    tablet.send(false).unwrap();
    let expected = if detach { 2 } else { 1 };
    destroyed(expected).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    complete(&native).await;
    reaped().await;
    assert_eq!(
        held_counts,
        (1, 0),
        "T1439 transition destroyed or replaced an unsettled payload"
    );
    assert_eq!(
        held_calls, before,
        "T1439 successor commands overtook original mapper"
    );
    assert_eq!(fixture::counts(), (expected, expected));
}

fn run_transition(name: &str, detach: bool) {
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(transition(detach));
}
#[test]
fn t1439_detach_and_latest_reattach_wait_for_original_payload_retirement() {
    run_transition(
        "t1439_detach_and_latest_reattach_wait_for_original_payload_retirement",
        true,
    );
}
#[test]
fn t1439_remap_waits_for_original_commands_before_reading_or_writing_again() {
    run_transition(
        "t1439_remap_waits_for_original_commands_before_reading_or_writing_again",
        false,
    );
}

async fn abandon_for_exit(
    started_tx: std::sync::mpsc::Sender<blent_config::retirement::Completion>,
) {
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(true);
    let backend = LinuxBackend::new(card).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablet, mode, config()));
    started().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    started_tx.send(native).unwrap();
}
struct ExitWorker {
    release: Option<fixture::Release>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl ExitWorker {
    fn settle(&mut self) {
        drop(self.release.take());
        self.thread.take().unwrap().join().unwrap();
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
    let release = fixture::Release::new();
    let (started_tx, original) = std::sync::mpsc::channel();
    let (finished, returned) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        blent::runtime_owner::run(abandon_for_exit(started_tx)).unwrap();
        finished.send(()).unwrap();
    });
    let mut worker = ExitWorker {
        release: Some(release),
        thread: Some(thread),
    };
    let proof = original.recv().unwrap();
    let early = returned.recv_timeout(Duration::from_millis(1200)).is_ok();
    let held_counts = fixture::counts();
    let native_pending = !proof.is_complete();
    worker.settle();
    assert!(
        !early,
        "T1439 process exit preceded original mapping and payload retirement"
    );
    assert!(native_pending);
    assert_eq!(held_counts, (1, 0));
    assert!(proof.is_complete());
    assert_eq!(fixture::counts(), (1, 1));
    assert!(fixture::calls().contains("mode i 1"));
}
#[test]
fn t1439_process_exit_waits_for_mapping_device_destruction_and_osk_restoration() {
    let name = "t1439_process_exit_waits_for_mapping_device_destruction_and_osk_restoration";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    process_exit();
}

async fn unknown_mapping_failure() {
    let release = fixture::Release::new();
    let (_card, card) = watch::channel(None);
    let (_tablet, tablet) = watch::channel(true);
    let (_mode, mode) = watch::channel(true);
    let backend = LinuxBackend::new(card.clone()).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablet, mode.clone(), config()));
    started().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    drop(release);
    reaped().await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let contender = LinuxBackend::new(card).unwrap();
    let contender_native = contender.native_completion().unwrap();
    let (_absent, absent) = watch::channel(false);
    let mut replacement = contender.follow(absent, mode, config());
    let rejected = futures_util::poll!(&mut replacement).is_ready();
    drop((replacement, contender));
    complete(&contender_native).await;
    assert_eq!(
        fixture::counts(),
        (1, 0),
        "T1439 unknown remote mutation released native identity"
    );
    assert!(!native.is_complete());
    assert!(rejected);
    // The isolated process owns only fake-uinput files. Its bounded quarantine
    // intentionally persists to process teardown; the original child is reaped.
}

#[test]
fn t1439_unknown_mapping_outcome_quarantines_payload_after_original_child_exit() {
    let name = "t1439_unknown_mapping_outcome_quarantines_payload_after_original_child_exit";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "7");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(unknown_mapping_failure());
}

async fn saturated_cleanup() {
    let release = fixture::Release::new();
    let (_card, card) = watch::channel(None);
    let (tablet, tablets) = watch::channel(true);
    let (_mode, modes) = watch::channel(true);
    let backend = LinuxBackend::new(card).unwrap();
    let native = backend.native_completion().unwrap();
    let task = tokio::spawn(backend.follow(tablets, modes, config()));
    started().await;
    let mut occupied = Vec::new();
    while let Ok(slot) = blent::work_admission::reserve() {
        occupied.push(slot);
    }
    assert!(!occupied.is_empty());
    tablet.send(false).unwrap();
    drop(release);
    destroyed(1).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let restored_while_full = fixture::calls().contains("mode i 1");
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    complete(&native).await;
    reaped().await;
    drop(occupied);
    assert!(
        restored_while_full,
        "T1439 payload cleanup lost OSK restoration to unrelated task admission"
    );
}

#[test]
fn t1439_detach_restores_keyboard_with_unrelated_task_admission_saturated() {
    let name = "t1439_detach_restores_keyboard_with_unrelated_task_admission_saturated";
    if std::env::var_os("BLENT_T1439_RELEASE").is_none() {
        fixture::run(name, "0");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(saturated_cleanup());
}
