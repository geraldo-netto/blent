//! T1433: canceled OSK commands retain original input and shared keyboard ownership.
#![cfg(target_os = "linux")]
use blent::input::{backend::InputBackend, InputConfig, LinuxBackend};
use std::{path::PathBuf, time::Duration};
use tokio::sync::watch;
#[path = "input_commands/fixture.rs"]
mod fixture;

async fn wait_for_file(path: &std::path::Path) {
    while std::fs::read_to_string(path).unwrap_or_default().is_empty() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn complete(proof: &blent_config::retirement::Completion) {
    while !proof.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn cancellation() {
    let release = fixture::Release::new();
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
    let child = PathBuf::from(std::env::var_os("BLENT_T1433_CHILD").unwrap());
    wait_for_file(&child).await;
    let pid = std::fs::read_to_string(&child).unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    let restore = tokio::spawn(blent::osk::Restoration::new().unwrap().run());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let early_completion = proof.is_complete();
    let early_restore = restore.is_finished();
    let calls = fixture::calls();
    let original_alive = PathBuf::from(format!("/proc/{pid}")).exists();
    drop(release);
    restore.await.unwrap();
    complete(&proof).await;
    while PathBuf::from(format!("/proc/{pid}")).exists() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    println!("original_alive={original_alive} input_complete_early={early_completion} restore_complete_early={early_restore} commands_before_release={calls}");
    assert!(
        original_alive,
        "fixture did not retain the original command"
    );
    assert!(
        !early_completion,
        "T1433 input retired before original command"
    );
    assert!(
        !early_restore,
        "T1433 shared operation gate released before original command"
    );
    assert_eq!(calls, 1, "T1433 successor command overtook the original");
}

#[test]
fn t1433_cancelled_osk_retains_input_and_shared_restore_gate() {
    let name = "t1433_cancelled_osk_retains_input_and_shared_restore_gate";
    if std::env::var_os("BLENT_T1433_CHILD").is_none() {
        fixture::run(name, "64");
        return;
    }
    fixture::intercepted();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(cancellation());
}

async fn abandon_for_exit(started: std::sync::mpsc::Sender<blent_config::retirement::Completion>) {
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
    wait_for_file(&PathBuf::from(
        std::env::var_os("BLENT_T1433_CHILD").unwrap(),
    ))
    .await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(backend);
    started.send(proof).unwrap();
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
    let (started, original) = std::sync::mpsc::channel();
    let (finished, returned) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        blent::runtime_owner::run(abandon_for_exit(started)).unwrap();
        finished.send(()).unwrap();
    });
    let mut worker = ExitWorker {
        release: Some(release),
        thread: Some(thread),
    };
    let proof = original.recv().unwrap();
    let early = returned.recv_timeout(Duration::from_millis(1200)).is_ok();
    let native_pending = !proof.is_complete();
    worker.settle();
    assert!(
        !early,
        "T1433 process returned before its original OSK command"
    );
    assert!(native_pending);
    assert!(proof.is_complete());
    let backup =
        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".local/share/blent/osk-restore");
    assert!(
        !backup.exists(),
        "T1433 final restoration rejected after process cutoff"
    );
    assert!(fixture::calls() > 1, "final restoration did not run");
}

#[test]
fn t1433_process_cutoff_preserves_admitted_input_osk_cleanup() {
    let name = "t1433_process_cutoff_preserves_admitted_input_osk_cleanup";
    if std::env::var_os("BLENT_T1433_CHILD").is_none() {
        fixture::run(name, "64");
        return;
    }
    fixture::intercepted();
    process_exit();
}

#[test]
fn t1433_one_task_slot_still_funds_original_osk_final_restoration() {
    let name = "t1433_one_task_slot_still_funds_original_osk_final_restoration";
    if std::env::var_os("BLENT_T1433_CHILD").is_none() {
        fixture::run(name, "1");
        return;
    }
    fixture::intercepted();
    process_exit();
}
