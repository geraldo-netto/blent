//! T1031: command results are distinct from original native completion.
use super::*;
use crate::retirement::Completion;
use std::sync::Mutex;

fn completed(proofs: &[Completion]) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while proofs.iter().any(|proof| !proof.is_complete()) {
        assert!(
            std::time::Instant::now() < end,
            "T1031 original command cleanup lost"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn t1031_sync_reports_original_collector_and_child_on_success_or_timeout() {
    let root = tempfile::tempdir().unwrap();
    for role in ["input", "wait"] {
        let observed = Mutex::new(Vec::new());
        let observe = |proof| observed.lock().unwrap().push(proof);
        let result = tests::fixture(&root.path().join(role), role)
            .output_timeout_observed(observation_budget(role), &observe);
        assert_eq!(result.is_ok(), role == "input", "T1031 {role}: {result:?}");
        let proofs = observed.into_inner().unwrap();
        assert_eq!(proofs.len(), 2, "T1031 sync lost original command proofs");
        completed(&proofs);
    }
}
#[test]
fn t1031_sync_spawn_failure_still_reports_exposed_collector() {
    let proofs = Mutex::new(Vec::new());
    let observe = |proof| proofs.lock().unwrap().push(proof);
    let root = tempfile::tempdir().unwrap();
    assert!(Command::new(root.path().join("missing"))
        .output_timeout_observed(Duration::from_secs(1), &observe)
        .is_err());
    let proofs = proofs.into_inner().unwrap();
    assert_eq!(proofs.len(), 1, "T1031 failed spawn lost collector proof");
    completed(&proofs);
}
#[tokio::test]
async fn t1031_async_reports_original_proofs_on_input_timeout_and_spawn_failure() {
    let root = tempfile::tempdir().unwrap();
    for role in ["input", "wait", "missing"] {
        let observed = Mutex::new(Vec::new());
        let observe = |proof| observed.lock().unwrap().push(proof);
        let mut command = match role {
            "missing" => tokio::process::Command::new(root.path().join("missing")),
            _ => tokio::process::Command::from(tests::fixture(&root.path().join(role), role)),
        };
        let result = command
            .output_input_timeout_observed(
                Some(b"T1031 secret"),
                observation_budget(role),
                &observe,
            )
            .await;
        assert_eq!(result.is_ok(), role == "input", "T1031 {role}: {result:?}");
        if role == "input" {
            assert_eq!(
                std::fs::read(root.path().join(role).with_extension("input")).unwrap(),
                b"T1031 secret"
            );
        }
        let proofs = observed.into_inner().unwrap();
        assert_eq!(
            proofs.len(),
            if role == "missing" { 1 } else { 2 },
            "T1031 async lost original command proofs"
        );
        completed(&proofs);
    }
}
// T1139: successful input is not a 100 ms native executable startup claim.
fn observation_budget(role: &str) -> Duration {
    if role == "input" || role == "input-delayed" {
        Duration::from_secs(2)
    } else {
        Duration::from_millis(100)
    }
}

#[tokio::test]
async fn t1139_delayed_startup_preserves_success_and_short_timeout_proofs() {
    for short in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("input");
        let observed = Mutex::new(Vec::new());
        let observe = |proof| observed.lock().unwrap().push(proof);
        let budget = if short {
            Duration::from_millis(100)
        } else {
            observation_budget("input-delayed")
        };
        let result = tokio::process::Command::from(tests::fixture(&path, "input-delayed"))
            .output_input_timeout_observed(Some(b"T1139 controlled input"), budget, &observe)
            .await;
        let proofs = observed.into_inner().unwrap();
        completed(&proofs);
        assert_eq!(
            proofs.len(),
            2,
            "T1139 original collector/child proofs lost"
        );
        assert_eq!(result.is_ok(), !short, "T1139 short={short}: {result:?}");
        if short {
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        } else {
            assert_eq!(
                std::fs::read(path.with_extension("input")).unwrap(),
                b"T1139 controlled input"
            );
        }
    }
}

#[tokio::test]
async fn t1031_cancelled_async_command_keeps_exact_original_proofs() {
    let proofs = Mutex::new(Vec::new());
    let observe = |proof| proofs.lock().unwrap().push(proof);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pid");
    let mut command = tokio::process::Command::from(tests::fixture(&path, "wait"));
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        command.output_input_timeout_observed(None, Duration::from_secs(10), &observe),
    )
    .await;
    assert!(result.is_err());
    let proofs = proofs.into_inner().unwrap();
    assert_eq!(
        proofs.len(),
        2,
        "T1031 cancellation lost original command proofs"
    );
    completed(&proofs);
}

#[test]
fn t1031_panicking_sync_observer_cannot_leak_exposed_child() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pid");
    let proofs = Mutex::new(Vec::new());
    let observe = |proof| {
        let mut proofs = proofs.lock().unwrap();
        proofs.push(proof);
        if proofs.len() == 2 {
            drop(proofs);
            let end = std::time::Instant::now() + Duration::from_secs(2);
            while !path.exists() {
                assert!(std::time::Instant::now() < end);
                std::thread::sleep(Duration::from_millis(1));
            }
            panic!("T1031 observer failure after actual child exposure");
        }
    };
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tests::fixture(&path, "denied").output_timeout_observed(Duration::from_secs(5), &observe)
    }));
    assert!(failed.is_err());
    let proofs = proofs.into_inner().unwrap();
    assert_eq!(proofs.len(), 2);
    completed(&proofs);
    let pid = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    tests::assert_exited(pid);
    assert!(!path.with_extension("done").exists());
}

#[tokio::test]
async fn t1031_unrepresentable_deadlines_reject_before_native_exposure() {
    let proofs = Mutex::new(Vec::new());
    let observe = |proof| proofs.lock().unwrap().push(proof);
    for seconds in [u64::MAX, u64::MAX - 1, u64::MAX / 2] {
        let timeout = Duration::from_secs(seconds);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.arg("--list");
        assert_eq!(
            command
                .output_timeout_observed(timeout, &observe)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let mut command = tokio::process::Command::from(command);
        assert_eq!(
            command
                .output_input_timeout_observed(None, timeout, &observe)
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert!(proofs.into_inner().unwrap().is_empty());
}
