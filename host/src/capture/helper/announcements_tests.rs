//! T1022: bound announcements and surface failure without releasing native ownership.
use super::*;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn t1022_oversized_announcement_cannot_precede_successful_handshake() {
    let mut child = Child::spawn(
        Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped()),
    )
    .unwrap();
    let mut input = child.take_stdin().unwrap();
    let mut bytes = vec![b'x'; 1025];
    bytes.extend_from_slice(b"\nEVDI_CONNECTED card17\n");
    input.write_all(&bytes).await.unwrap();
    drop(input);
    let mut lines = HelperLines::new(child.take_stdout().unwrap());
    let result = HelperProcess::await_helper_card(&mut lines).await;
    let status = child.finish(Duration::from_secs(2)).await.unwrap();
    assert!(status.success());
    assert!(
        result.is_err(),
        "T1022 oversized helper line bypassed framing bound"
    );
}

async fn failed_announcements(script: &str) {
    let directory = tempfile::tempdir().unwrap();
    let mut child = Child::spawn(
        Command::new("/bin/sh")
            .args(["-c", script])
            .stdout(Stdio::piped()),
    )
    .unwrap();
    let original = child.completion();
    let report = blent_config::linux::pipe::ReportFile::new(
        directory.path().join("frames"),
        child.id().unwrap(),
    )
    .unwrap();
    let mut helper = HelperProcess::new();
    let lines = HelperLines::new(child.take_stdout().unwrap());
    let work = HelperProcess::drain_helper_stdout(
        lines,
        helper.mode_tx.clone(),
        helper.stream_tx.clone(),
        helper.fifo_reset_tx.clone(),
        report,
    );
    helper.stdout_task = Some(blent::owned_task::Task::spawn(work).unwrap());
    helper.child = Some(child);
    let outcome = tokio::time::timeout(Duration::from_millis(200), helper.wait()).await;
    let repeated = tokio::time::timeout(Duration::from_millis(10), helper.wait()).await;
    assert!(
        !original.is_complete(),
        "T1022 task completion released live child"
    );
    assert!(helper.child.is_some());
    helper.stop();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(outcome
        .expect("T1022 stdout failure did not wake capture owner")
        .is_err());
    assert!(repeated
        .expect("T1022 failed stdout became live again")
        .is_err());
}

#[tokio::test]
async fn t1022_transport_failure_wakes_original_native_owner() {
    for script in [
        "printf '\\377\\n'; exec sleep 60",
        "exec 1>&-; exec sleep 60",
    ] {
        failed_announcements(script).await;
    }
}

#[tokio::test]
async fn t1022_wait_without_stdout_retains_native_status_contract() {
    let mut helper = HelperProcess::new();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), helper.wait())
            .await
            .is_err()
    );
    let child = Child::spawn(&mut Command::new("/bin/true")).unwrap();
    let original = child.completion();
    helper.child = Some(child);
    assert!(helper.wait().await.unwrap().success());
    helper.terminate().await;
    assert!(original.is_complete());
}
