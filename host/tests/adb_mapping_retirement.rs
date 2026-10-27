//! T1090 actual adapter/child-process regressions; never invoke physical ADB.
#![cfg(target_os = "linux")]
#[path = "../src/adb_command.rs"]
mod adb_command;
#[path = "../src/adb_mapping/mod.rs"]
mod adb_mapping;
#[path = "../src/camera/bridge.rs"]
mod bridge;
#[path = "adb_mapping/stop.rs"]
mod camera_stop;

use blent_config::retirement::Completion;
use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

struct Fixture {
    root: tempfile::TempDir,
    proof: Arc<Mutex<Option<Completion>>>,
    server: std::process::Child,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.server.kill();
        self.server.wait().unwrap();
    }
}
impl Fixture {
    fn path(&self) -> &Path {
        self.root.path()
    }
    fn observer(&self) -> impl Fn(Completion) -> anyhow::Result<()> + Send + Sync + 'static {
        let proof = self.proof.clone();
        move |completion| {
            *proof.lock().unwrap() = Some(completion);
            Ok(())
        }
    }
    fn retired(&self) -> bool {
        self.proof
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(Completion::is_complete)
    }
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, include_str!("fixtures/adb_mapping.py")).unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let server = std::process::Command::new(&adb)
        .arg("--server")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    Fixture {
        root,
        proof: Arc::new(Mutex::new(None)),
        server,
    }
}

async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("T1090 expected native progress did not occur");
}

fn empty(root: &Path) -> bool {
    std::fs::read_to_string(root.join("routes.json")).is_ok_and(|text| text == "{}")
}

async fn listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap()
}

#[tokio::test]
async fn t1090_drop_retires_the_original_mapping() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    assert!(!empty(root.path()));
    drop(bridge);
    until(|| empty(root.path())).await;
    until(|| root.retired()).await;
}

#[tokio::test]
async fn t1090_failed_creation_cleans_its_partial_native_effect() {
    let root = fixture();
    std::fs::write(root.path().join("fail_create"), "").unwrap();
    assert!(bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer()
    )
    .await
    .is_err());
    until(|| empty(root.path())).await;
    until(|| root.retired()).await;
}

#[tokio::test]
async fn t1090_cancelled_creation_retains_its_late_creator_and_mapping() {
    let root = fixture();
    let hold = root.path().join("hold_create");
    std::fs::write(&hold, "").unwrap();
    let adb = root.path().join("adb");
    let observer = root.observer();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let destination = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _original_listener = listener;
        bridge::Bridge::create(adb, Some("original"), &_original_listener, &observer).await
    });
    until(|| root.path().join("created").exists()).await;
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    assert!(!empty(root.path()));
    assert!(!root.retired());
    let destination_retained = std::net::TcpListener::bind(destination).is_err();
    std::fs::remove_file(hold).unwrap();
    until(|| empty(root.path())).await;
    until(|| root.retired()).await;
    assert!(
        destination_retained,
        "T1090 cancelled creator released its local destination before mapping cleanup"
    );
    drop(std::net::TcpListener::bind(destination).unwrap());
}

#[tokio::test]
async fn t1090_cleanup_error_keeps_the_original_obligation_after_drop() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let denied = root.path().join("fail_remove");
    std::fs::write(&denied, "").unwrap();
    assert!(bridge.close().await.is_err());
    drop(bridge);
    assert!(!empty(root.path()));
    assert!(!root.retired());
    std::fs::remove_file(denied).unwrap();
    until(|| empty(root.path())).await;
    until(|| root.retired()).await;
}

#[tokio::test]
async fn t1090_unknown_remove_is_not_reissued_before_its_delegated_effect() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let denied = root.path().join("fail_remove");
    std::fs::write(&denied, "").unwrap();
    assert!(bridge.close().await.is_err());
    tokio::time::sleep(Duration::from_millis(250)).await;
    let calls: Vec<Vec<String>> = std::fs::read_to_string(root.path().join("calls.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        calls
            .iter()
            .filter(|args| args.iter().any(|arg| arg == "--remove"))
            .count(),
        1
    );
    assert!(!root.retired());
    drop(bridge);
    std::fs::remove_file(denied).unwrap();
    until(|| root.retired()).await;
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1090_empty_snapshot_cannot_retire_a_delegated_late_creation() {
    let root = fixture();
    let deferred = root.path().join("defer_create");
    std::fs::write(&deferred, "").unwrap();
    std::fs::write(root.path().join("routes.json"), "{}").unwrap();
    assert!(bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer()
    )
    .await
    .is_err());
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(empty(root.path()));
    assert!(!root.retired());
    std::fs::remove_file(deferred).unwrap();
    until(|| root.retired()).await;
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1090_invitation_failure_and_cancelled_close_keep_command_fences() {
    for flag in ["reject_shell", "fail_shell"] {
        let root = fixture();
        let bridge = bridge::Bridge::create(
            root.path().join("adb"),
            Some("original"),
            &listener().await,
            &root.observer(),
        )
        .await
        .unwrap();
        std::fs::write(root.path().join(flag), "").unwrap();
        assert!(bridge
            .invite(
                &"ab".repeat(32),
                &blent_config::camera::CameraProfile::default()
            )
            .await
            .is_err());
        bridge.close().await.unwrap();
        assert!(root.retired());
    }
    cancelled_invitation().await;
}

async fn cancelled_invitation() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let hold = root.path().join("hold_shell");
    std::fs::write(&hold, "").unwrap();
    let token = "ab".repeat(32);
    let profile = blent_config::camera::CameraProfile::default();
    let mut invitation = Box::pin(bridge.invite(&token, &profile));
    tokio::select! {
        result = &mut invitation => panic!("T1090 invitation unexpectedly completed: {result:?}"),
        () = until(|| root.path().join("shell_entered").exists()) => {}
    }
    drop(invitation);
    let mut closing = Box::pin(bridge.close());
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut closing)
            .await
            .is_err()
    );
    drop(closing);
    assert!(!root.retired());
    assert!(!root.path().join("remove_attempt").exists());
    std::fs::remove_file(hold).unwrap();
    bridge.close().await.unwrap();
    assert!(root.retired());
}

#[test]
fn t1090_runtime_teardown_preserves_the_independent_cleanup_sponsor() {
    let root = fixture();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = runtime.block_on(listener());
    let bridge = runtime
        .block_on(bridge::Bridge::create(
            root.path().join("adb"),
            Some("original"),
            &local,
            &root.observer(),
        ))
        .unwrap();
    drop(runtime);
    drop(local);
    drop(bridge);
    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    while !root.retired() {
        assert!(
            std::time::Instant::now() < deadline,
            "T1090 independent sponsor did not retire"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(empty(root.path()));
}

#[test]
fn t1090_exhaustion_keeps_admission_until_original_cleanup_and_join() {
    if std::env::var_os("BLENT_T1090_CAPACITY_CHILD").is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(capacity_case());
        return;
    }
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "t1090_exhaustion_keeps_admission_until_original_cleanup_and_join",
            "--nocapture",
        ])
        .env("BLENT_T1090_CAPACITY_CHILD", "1")
        .env("BLENT_ADB_MAPPING_LIMIT", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn capacity_case() {
    let root = fixture();
    let adb = root.path().join("adb");
    let bridge = bridge::Bridge::create(
        adb.clone(),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let denied = root.path().join("fail_remove");
    std::fs::write(&denied, "").unwrap();
    assert!(bridge.close().await.is_err());
    drop(bridge);
    let before = std::fs::read(root.path().join("calls.jsonl")).unwrap();
    assert!(bridge::Bridge::create(
        adb.clone(),
        Some("original"),
        &listener().await,
        &|_| Ok(())
    )
    .await
    .is_err());
    let after = std::fs::read(root.path().join("calls.jsonl")).unwrap();
    assert!(!root.retired());
    // No second mutating command may appear; the original worker can still poll.
    assert_eq!(creation_count(&before), creation_count(&after));
    std::fs::remove_file(denied).unwrap();
    until(|| root.retired()).await;
    let successor =
        bridge::Bridge::create(adb, Some("original"), &listener().await, &root.observer())
            .await
            .unwrap();
    successor.close().await.unwrap();
    assert!(root.retired());
}

fn creation_count(calls: &[u8]) -> usize {
    String::from_utf8_lossy(calls)
        .lines()
        .filter(|line| line.contains("--no-rebind"))
        .count()
}

#[tokio::test]
async fn t1090_invalid_input_and_rejected_observation_have_no_mapping_effects() {
    let root = fixture();
    assert!(blent_config::adb_reverse::temporary::port("tcp:0").is_err());
    for serial in ["", " \n"] {
        assert!(bridge::Bridge::create(
            root.path().join("adb"),
            Some(serial),
            &listener().await,
            &root.observer()
        )
        .await
        .is_err());
    }
    assert!(bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &|_| anyhow::bail!("T1090 observation rejected")
    )
    .await
    .is_err());
    assert!(!root.path().join("calls.jsonl").exists());
    assert!(bridge::Bridge::create(
        root.path().join("absent"),
        Some("original"),
        &listener().await,
        &root.observer()
    )
    .await
    .is_err());
    until(|| root.retired()).await;
}

#[tokio::test]
async fn t1090_unstarted_removal_can_retry_without_inventing_a_pending_effect() {
    let root = fixture();
    let adb = root.path().join("adb");
    let bridge = bridge::Bridge::create(
        adb.clone(),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    std::fs::write(root.path().join("hide_after_list"), "").unwrap();
    assert!(bridge.close().await.is_err());
    assert!(!root.retired());
    assert!(!root.path().join("remove_pending").exists());
    std::fs::rename(root.path().join("adb.saved"), adb).unwrap();
    drop(bridge);
    until(|| root.retired()).await;
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1090_unstarted_creation_settles_without_deleting_any_route() {
    let root = fixture();
    std::fs::write(root.path().join("hide_after_list"), "").unwrap();
    assert!(bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer()
    )
    .await
    .is_err());
    until(|| root.retired()).await;
    assert!(!root.path().join("created").exists());
    assert!(!root.path().join("remove_attempt").exists());
}

#[tokio::test]
async fn t1090_native_deadline_does_not_abandon_a_partial_mapping() {
    let root = fixture();
    std::fs::write(root.path().join("hold_create"), "").unwrap();
    let error = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await;
    assert!(error.is_err());
    assert!(root.path().join("created").exists());
    until(|| root.retired()).await;
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1090_owned_invitation_checks_the_full_native_acknowledgement_oracle() {
    let root = fixture();
    let adb = root.path().join("adb");
    let bridge = bridge::Bridge::create(
        adb.clone(),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let original = std::fs::read(&adb).unwrap();
    let token = "ab".repeat(32);
    let mut observed = Vec::new();
    for (stdout, expected) in adb_command::broadcast_tests::cases() {
        adb_command::broadcast_tests::fixture(root.path(), &stdout, true);
        let actual = bridge
            .invite(&token, &blent_config::camera::CameraProfile::default())
            .await;
        observed.push((actual.is_ok(), expected));
    }
    std::fs::write(&adb, original).unwrap();
    bridge.close().await.unwrap();
    assert!(root.retired());
    assert!(observed.iter().all(|(actual, expected)| actual == expected));
}

#[tokio::test]
async fn t1090_retired_cycles_release_native_worker_and_listener_descriptors() {
    retired_cycle().await;
    let baseline = std::fs::read_dir("/proc/self/fd").unwrap().count();
    for _ in 0..4 {
        retired_cycle().await;
        assert_eq!(
            std::fs::read_dir("/proc/self/fd").unwrap().count(),
            baseline,
            "T1090 settled mapping retained a native descriptor"
        );
    }
}

async fn retired_cycle() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    bridge.close().await.unwrap();
    drop(bridge);
    assert!(root.retired());
}

#[tokio::test]
async fn t1090_malformed_cleanup_snapshot_retains_ownership_until_recovery() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let invalid = root.path().join("invalid_list");
    std::fs::write(&invalid, "").unwrap();
    assert!(bridge.close().await.is_err());
    drop(bridge);
    assert!(!root.retired());
    assert!(!root.path().join("remove_attempt").exists());
    std::fs::remove_file(invalid).unwrap();
    until(|| root.retired()).await;
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1090_bounded_invitation_queue_rejects_excess_and_cuts_off_queued_work() {
    let root = fixture();
    let bridge = adb_mapping::Lease::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    assert!(bridge.shell("x".repeat(4097)).await.is_err());
    let hold = root.path().join("hold_shell");
    std::fs::write(&hold, "").unwrap();
    let command = "T1090 private invitation\n".to_owned();
    let mut first = Box::pin(bridge.shell(command.clone()));
    tokio::select! {
        result = &mut first => panic!("T1090 first invitation completed: {result:?}"),
        () = until(|| root.path().join("shell_entered").exists()) => {}
    }
    let mut queued = Box::pin(bridge.shell(command.clone()));
    assert!(tokio::time::timeout(Duration::from_millis(30), &mut queued)
        .await
        .is_err());
    assert!(bridge.shell(command.clone()).await.is_err());
    drop(queued);
    drop(first);
    let mut closing = Box::pin(bridge.close());
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut closing)
            .await
            .is_err()
    );
    drop(closing);
    std::fs::remove_file(hold).unwrap();
    bridge.close().await.unwrap();
    let calls = std::fs::read_to_string(root.path().join("calls.jsonl")).unwrap();
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.contains("\"shell\""))
            .count(),
        1
    );
    assert!(root.retired());
}

#[tokio::test]
async fn t1090_missing_and_foreign_routes_never_authorize_removal() {
    for replacement in [None, Some("tcp:54321")] {
        let root = fixture();
        let bridge = bridge::Bridge::create(
            root.path().join("adb"),
            Some("original"),
            &listener().await,
            &root.observer(),
        )
        .await
        .unwrap();
        let remote = std::fs::read_to_string(root.path().join("created")).unwrap();
        let routes = replacement
            .map(|local| serde_json::json!({remote: local}))
            .unwrap_or(serde_json::json!({}));
        std::fs::write(root.path().join("routes.json"), routes.to_string()).unwrap();
        bridge.close().await.unwrap();
        drop(bridge);
        assert!(root.retired());
        assert!(!root.path().join("remove_attempt").exists());
        assert_eq!(
            std::fs::read_to_string(root.path().join("routes.json")).unwrap(),
            routes.to_string()
        );
    }
}

#[tokio::test]
async fn t1090_repeated_close_and_invitation_cutoff_are_idempotent() {
    let root = fixture();
    let bridge = bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap();
    let token = "ab".repeat(32);
    let endpoint = format!("127.0.0.1:{}", bridge.target().port().unwrap());
    assert!(std::net::TcpListener::bind(&endpoint).is_err());
    bridge
        .invite(&token, &blent_config::camera::CameraProfile::default())
        .await
        .unwrap();
    let (first, second) = tokio::join!(bridge.close(), bridge.close());
    first.unwrap();
    second.unwrap();
    assert!(root.retired());
    drop(
        std::net::TcpListener::bind(&endpoint)
            .expect("T1090 completion retained the endpoint reservation"),
    );
    assert!(bridge
        .invite(&token, &blent_config::camera::CameraProfile::default())
        .await
        .is_err());
    bridge.close().await.unwrap();
}
