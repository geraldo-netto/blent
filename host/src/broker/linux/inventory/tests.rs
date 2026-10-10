use super::*;
use std::{future::Future, os::unix::fs::PermissionsExt, time::Duration};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .unwrap()
}
fn helper(root: &std::path::Path, body: &str) -> PathBuf {
    let path = root.join("inventory-fixture");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[tokio::test]
async fn t853_cancelled_terminal_wait_keeps_inventory_result_and_join_owned() {
    let (release, held) = tokio::sync::oneshot::channel::<()>();
    let task = Task::spawn(async move {
        let _ = held.await;
        Ok(())
    })
    .unwrap();
    let (output, events) = mpsc::channel(2);
    output.send(Event::Finished(Ok(Vec::new()))).await.unwrap();
    let mut inventory = Inventory {
        task,
        events,
        native: None,
        finished: false,
        terminal: None,
    };
    assert!(
        tokio::time::timeout(Duration::from_millis(10), inventory.next())
            .await
            .is_err()
    );
    assert!(!inventory.finished(), "cancelled join lost terminal result");
    release.send(()).unwrap();
    assert!(bounded(inventory.next()).await.unwrap().is_empty());
    assert!(inventory.retired());
}
#[tokio::test]
async fn t853_inventory_terminal_notice_observes_actual_task_retirement() {
    let root = tempfile::tempdir().unwrap();
    let path = helper(
        root.path(),
        "printf 'BLUI\\000\\000\\000\\001\\000\\000\\000\\000\\000\\000\\000\\000'",
    );
    let mut inventory = Inventory::start(path, Limits::default()).unwrap();
    assert!(!inventory.finished());
    assert!(!inventory.retired());
    assert!(bounded(inventory.next()).await.unwrap().is_empty());
    assert!(inventory.finished());
    assert!(inventory.next().await.is_err());
    bounded(async {
        while !inventory.retired() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}
#[tokio::test]
async fn t853_inventory_spawn_and_read_failures_stay_bounded() {
    let mut missing = Inventory::start(
        PathBuf::from("/missing/blent-inventory-fixture"),
        Limits::default(),
    )
    .unwrap();
    assert!(bounded(missing.next()).await.is_err());
    bounded(async {
        while !missing.retired() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let path = helper(root.path(), "exec sleep 30");
    let mut stalled = Inventory::start(
        path,
        Limits {
            timeout: Duration::from_millis(30),
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(bounded(stalled.next()).await.is_err());
    bounded(async {
        while !stalled.retired() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}
