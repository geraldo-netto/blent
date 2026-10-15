//! T1020: startup cancellation cannot release original native cleanup authority.
use super::*;
use crate::capture::ownership_tests::isolated;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;

struct HeldCleanup {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Drop for HeldCleanup {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
    }
}
struct Release(mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
fn configuration() -> CaptureConfig {
    use std::os::unix::fs::PermissionsExt;
    let root = crate::runtime::runtime_dir().unwrap();
    let path = root.join("t1020-helper");
    std::fs::write(&path, "#!/bin/sh\ncase \"$BLENT_T1020_MODE\" in\nsilent) exec sleep 60;;\nbad) printf 'EVDI_CONNECTED card-1\\n';;\n*) printf 'EVDI_CONNECTED card17\\n';;\nesac\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    CaptureConfig {
        helper_path: path,
        edid_path: Some(root.join("unused.edid")),
        encoder: "libx264".into(),
        ..Default::default()
    }
}
async fn completed(completion: &blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("T1020 original cleanup did not finish after release");
}
async fn exercise(mode: &str) {
    let config = configuration();
    let mut helper = HelperProcess::new();
    let (entered_tx, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let release = Release(release);
    let original = Arc::new(Mutex::new(None));
    let observed = original.clone();
    let outcome = tokio::time::timeout(
        Duration::from_millis(100),
        helper.start_with(&config, |command| {
            command.env("BLENT_T1020_MODE", mode);
            let mut child = Child::spawn(command)?;
            *observed.lock().unwrap() = Some(child.completion());
            child.keep_alive(HeldCleanup {
                entered: entered_tx,
                release: Mutex::new(held),
            });
            Ok(child)
        }),
    )
    .await;
    if mode == "silent" {
        assert!(outcome.is_err());
    } else {
        assert!(outcome.unwrap().is_err());
    }
    entered
        .recv_timeout(Duration::from_secs(5))
        .expect("T1020 original process was not reaped into held cleanup");
    let original = original.lock().unwrap().clone().unwrap();
    let retained = helper
        .pending
        .as_ref()
        .expect("T1020 startup lost original retirement certificate")
        .clone();
    assert!(!original.is_complete());
    assert!(!retained.is_complete());
    let spawns = AtomicUsize::new(0);
    let retry = helper
        .start_with(&config, |command| {
            spawns.fetch_add(1, Ordering::SeqCst);
            Child::spawn(command)
        })
        .await;
    assert!(retry.unwrap_err().to_string().contains("still retiring"));
    assert_eq!(
        spawns.load(Ordering::SeqCst),
        0,
        "T1020 replacement exposed before original cleanup"
    );
    drop(release);
    completed(&original).await;
    completed(&retained).await;
    helper.start(&config).await.unwrap();
    assert_eq!(helper.card, Some(17));
    helper.terminate().await;
}

#[tokio::test]
async fn t1020_cancelled_handshake_retains_original_native_cleanup_before_retry() {
    if isolated("capture::helper::startup_ownership_tests::t1020_cancelled_handshake_retains_original_native_cleanup_before_retry") { return; }
    exercise("silent").await;
}
#[tokio::test]
async fn t1020_failed_handshake_retains_original_native_cleanup_before_retry() {
    if isolated("capture::helper::startup_ownership_tests::t1020_failed_handshake_retains_original_native_cleanup_before_retry") { return; }
    exercise("bad").await;
}
