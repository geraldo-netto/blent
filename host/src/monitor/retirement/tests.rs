use super::*;
use blent_config::commands::SyncCommandExt;

#[test]
fn t919_retiring_slot_waits_for_native_exit_in_isolated_process() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "monitor::retirement::tests::t919_blocked_native_fixture",
            "--nocapture",
        ])
        .env("BLENT_T919_FIXTURE", "1")
        .env("BLENT_SESSION_STOP_MS", "10")
        .env("BLENT_REAP_TIMEOUT_MS", "10")
        .output_timeout(Duration::from_secs(8))
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
}
#[tokio::test]
async fn t919_blocked_native_fixture() {
    if std::env::var_os("BLENT_T919_FIXTURE").is_none() {
        return;
    }
    let mut state = super::super::tests::fixture();
    let (release, hold) = std::sync::mpsc::channel();
    let (started, entered) = tokio::sync::oneshot::channel();
    let admission = blent::session::Workers::reserve().unwrap();
    let capture = tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        hold.recv_timeout(Duration::from_secs(4)).unwrap();
        Ok(())
    });
    entered.await.unwrap();
    let (stop, _) = watch::channel(false);
    let runtime = session::Runtime::from_workers(
        1,
        state.config.tablet.clone(),
        Default::default(),
        stop,
        blent::session::Workers::new(admission, capture, vec![]),
        (0, 0),
    );
    let completion = runtime.retirement.clone();
    state.retire("blocked".into(), 1, Some(runtime), None);
    let early = tokio::time::timeout(Duration::from_millis(100), state.retiring.join_next()).await;
    let premature = early.is_ok();
    if let Ok(Some(Ok(retired))) = early {
        state.retired(retired);
    }
    let held = state.available_slot().is_none();
    assert!(!completion.is_complete());
    state.retire("sibling".into(), 0, None, None);
    let sibling = tokio::time::timeout(Duration::from_secs(1), state.retiring.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(sibling.0, "sibling");
    state.retired(sibling);
    release.send(()).unwrap();
    if !premature {
        let retired = tokio::time::timeout(Duration::from_secs(1), state.retiring.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        state.retired(retired);
    }
    assert!(
        !premature,
        "T919 monitor freed slot before native capture exited"
    );
    assert!(held, "T919 retiring instance was reusable");
    assert!(completion.is_complete());
    assert_eq!(state.available_slot(), Some(1));
    state.stop().await;
}
