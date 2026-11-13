//! T1444: retained execution has no execution deadline but keeps original bounds/proofs.
use super::*;
use crate::retirement::Completion;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
type Proofs = Arc<Mutex<Vec<Completion>>>;

fn child(root: &Path, role: &str) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "commands::retained_tests::t1444_native_child",
            "--nocapture",
        ])
        .env("BLENT_T1444_ROOT", root)
        .env("BLENT_T1444_ROLE", role);
    command
}

#[test]
fn t1444_native_child() {
    let Some(root) = std::env::var_os("BLENT_T1444_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    std::fs::write(root.join("started"), std::process::id().to_string()).unwrap();
    match std::env::var("BLENT_T1444_ROLE").unwrap().as_str() {
        "hold" => {
            while root.exists() && !root.join("release").exists() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        "input" => {
            use std::io::Read;
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            std::fs::write(root.join("input"), bytes).unwrap();
        }
        "flood" => {
            use std::io::Write;
            for _ in 0..4097 {
                std::io::stdout().write_all(&[7; 16384]).unwrap();
            }
        }
        _ => panic!("unexpected private fixture role"),
    }
    println!("T1444 settled");
}

async fn invoke(
    mut command: tokio::process::Command,
    input: Option<Vec<u8>>,
    proofs: Proofs,
) -> io::Result<Output> {
    let observe = |proof| {
        let mut proofs = proofs.lock().unwrap();
        assert!(proofs.len() < 2, "one original collector and child");
        proofs.push(proof);
    };
    command
        .output_input_retained_observed(input.as_deref(), &observe)
        .await
}
async fn started(root: &Path, task: &tokio::task::JoinHandle<io::Result<Output>>) {
    while !root.join("started").exists() {
        assert!(
            !task.is_finished(),
            "private command exited before fixture started"
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn settled(proofs: &Proofs) -> usize {
    while proofs
        .lock()
        .unwrap()
        .iter()
        .any(|proof| !proof.is_complete())
    {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    proofs.lock().unwrap().len()
}

async fn held_execution(root: &Path) {
    let proofs = Proofs::default();
    let task = tokio::spawn(invoke(child(root, "hold"), None, proofs.clone()));
    started(root, &task).await;
    // A finite fixture hold beyond the existing command protocol deadline; no work is killed here.
    tokio::time::sleep(COMMAND_TIMEOUT + Duration::from_millis(100)).await;
    let early = task.is_finished();
    let pending = proofs
        .lock()
        .unwrap()
        .iter()
        .all(|proof| !proof.is_complete());
    std::fs::write(root.join("release"), "release").unwrap();
    let result = task.await.unwrap();
    assert_eq!(settled(&proofs).await, 2);
    assert!(
        !early,
        "T1444 retained command expired before original execution completed"
    );
    assert!(
        pending,
        "original command/collector proofs released during retained execution"
    );
    let output = result.unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("T1444 settled"));
}
async fn input_and_spawn(root: &Path, missing: bool) {
    let proofs = Proofs::default();
    let command = if missing {
        tokio::process::Command::new(root.join("missing"))
    } else {
        child(root, "input")
    };
    let result = invoke(
        command,
        Some(b"T1444 private input".to_vec()),
        proofs.clone(),
    )
    .await;
    assert_eq!(settled(&proofs).await, if missing { 1 } else { 2 });
    if missing {
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    } else {
        assert!(result.unwrap().status.success());
        assert_eq!(
            std::fs::read(root.join("input")).unwrap(),
            b"T1444 private input"
        );
    }
}
async fn cancellation(root: &Path) {
    let proofs = Proofs::default();
    let task = tokio::spawn(invoke(child(root, "hold"), None, proofs.clone()));
    started(root, &task).await;
    task.abort();
    let result = task.await;
    std::fs::write(root.join("release"), "release").unwrap();
    assert_eq!(settled(&proofs).await, 2);
    assert!(result.unwrap_err().is_cancelled());
}
async fn output_bound(root: &Path) {
    let proofs = Proofs::default();
    let result = invoke(child(root, "flood"), None, proofs.clone()).await;
    assert_eq!(settled(&proofs).await, 2);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
}
async fn admission_bound(root: &Path) {
    let proofs = Proofs::default();
    let task = tokio::spawn(invoke(child(root, "hold"), None, proofs.clone()));
    started(root, &task).await;
    let rejected = Proofs::default();
    let second = invoke(child(root, "input"), None, rejected.clone()).await;
    std::fs::write(root.join("release"), "release").unwrap();
    let original = task.await.unwrap();
    assert_eq!(settled(&proofs).await, 2);
    assert!(original.unwrap().status.success());
    assert_eq!(second.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    assert_eq!(settled(&rejected).await, 0);
}
async fn scenario(root: &Path, mode: &str) {
    match mode {
        "hold" => held_execution(root).await,
        "input" => input_and_spawn(root, false).await,
        "missing" => input_and_spawn(root, true).await,
        "cancel" => cancellation(root).await,
        "flood" => output_bound(root).await,
        "capacity" => admission_bound(root).await,
        _ => panic!("unknown retained execution case"),
    }
}
#[test]
fn t1444_retained_fixture() {
    let Ok(mode) = std::env::var("BLENT_T1444_CASE") else {
        return;
    };
    let root = tempfile::tempdir().unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(scenario(root.path(), &mode));
}
#[test]
fn t1444_retained_execution_keeps_original_bounds_and_proofs() {
    for mode in ["hold", "input", "missing", "cancel", "flood", "capacity"] {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "commands::retained_tests::t1444_retained_fixture",
                "--nocapture",
            ])
            .env("BLENT_T1444_CASE", mode)
            .env("BLENT_COMMAND_COLLECTORS", "1")
            .env("BLENT_COMMAND_STDOUT_BYTES", "65536")
            .env("BLENT_COMMAND_STDERR_BYTES", "65536")
            .env("BLENT_COMMAND_TOTAL_BYTES", "65536");
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{mode}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
