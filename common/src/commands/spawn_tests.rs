//! T1638: exit receipts observe original children; they never own termination.
use super::*;
use crate::retirement::Observe;
use std::process::Command;

#[tokio::test]
async fn t1638_exit_receipt_reports_reaped_success_and_failure() {
    for program in ["/bin/true", "/bin/false"] {
        let mut child = spawn_reaped(&mut Command::new(program)).unwrap();
        let status =
            tokio::time::timeout(Duration::from_secs(2), child.exit.wait_for(Option::is_some))
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        assert_eq!(status.success(), program == "/bin/true");
        assert_eq!(
            status.code(),
            Some(if program == "/bin/true" { 0 } else { 1 })
        );
    }
    assert!(spawn_reaped(&mut Command::new("/blent-t1638-absent")).is_err());
}

#[test]
fn t1638_dropped_receipt_neither_kills_nor_releases_original_child() {
    let mut child = Command::new("/bin/cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let (report, receipt) = tokio::sync::watch::channel(None);
    let mut owner = retirement::SyncChild {
        child,
        report: Some(report),
    };
    drop(receipt);
    assert!(!owner.poll_complete());
    drop(input);
    await_reap(&mut owner);
    assert!(owner.report.is_none());
}

fn await_reap(owner: &mut retirement::SyncChild) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !owner.poll_complete() {
        if std::time::Instant::now() >= deadline {
            let _ = owner.child.kill();
            let _ = owner.child.wait();
            panic!("T1638 original child failed to settle");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn t1638_unobserved_retirement_uses_the_same_original_reap() {
    let child = Command::new("/bin/true").spawn().unwrap();
    let mut owner = retirement::SyncChild {
        child,
        report: None,
    };
    await_reap(&mut owner);
    assert!(owner.poll_complete());
}
