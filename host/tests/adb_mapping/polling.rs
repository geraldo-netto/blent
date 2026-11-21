//! T1657: real private commands, offline retention and bounded cleanup cadence.
use super::*;

fn calls(root: &Path) -> Vec<(f64, Vec<String>)> {
    std::fs::read_to_string(root.join("times.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

async fn bridge(root: &Fixture) -> bridge::Bridge {
    bridge::Bridge::create(
        root.path().join("adb"),
        Some("original"),
        &listener().await,
        &root.observer(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn t1657_absence_retains_endpoints_without_device_commands() {
    let root = fixture();
    let bridge = bridge(&root).await;
    let count = calls(root.path()).len();
    std::fs::write(root.path().join("absent"), "").unwrap();
    let rejected = bridge.close().await.is_err();
    tokio::time::sleep(Duration::from_millis(450)).await;
    let observations = calls(root.path())[count..].to_vec();
    let retained = !root.retired() && !empty(root.path());
    std::fs::remove_file(root.path().join("absent")).unwrap();
    drop(bridge);
    until(|| root.retired()).await;
    assert!(
        rejected && retained,
        "T1657 absent tablet released original ownership"
    );
    assert!(!observations.is_empty());
    assert!(observations.iter().all(|(_, args)| args == &["devices"]));
    assert!(empty(root.path()));
}

#[tokio::test]
async fn t1657_failed_cleanup_doubles_retry_delay() {
    let root = fixture();
    let bridge = bridge(&root).await;
    std::fs::write(root.path().join("invalid_list"), "").unwrap();
    let count = calls(root.path()).len();
    let rejected = bridge.close().await.is_err();
    until(|| {
        calls(root.path())[count..]
            .iter()
            .filter(|(_, args)| args.iter().any(|a| a == "--list"))
            .count()
            >= 4
    })
    .await;
    let observations = calls(root.path())[count..].to_vec();
    let retained = !root.retired();
    std::fs::remove_file(root.path().join("invalid_list")).unwrap();
    drop(bridge);
    until(|| root.retired()).await;
    assert!(rejected && retained);
    let timestamps: Vec<_> = observations
        .iter()
        .filter(|(_, args)| args.iter().any(|a| a == "--list"))
        .map(|(time, _)| *time)
        .collect();
    for (pair, minimum) in timestamps.windows(2).take(3).zip([0.1, 0.2, 0.4]) {
        assert!(
            pair[1] - pair[0] >= minimum,
            "T1657 retry did not back off: {timestamps:?}"
        );
    }
}

fn rejected_inventories() -> Vec<Vec<u8>> {
    vec![
        Vec::new(),
        b"List of devices attached\noriginal\tdevice\noriginal\toffline\n".to_vec(),
        b"List of devices attached\noriginal\tinvalid state\n".to_vec(),
        b"List of devices attached\nother\tdevice\n".to_vec(),
        vec![b'x'; 65537],
        vec![255],
    ]
}

#[tokio::test]
async fn t1657_invalid_inventory_retains_mapping_until_valid_observation() {
    for listing in rejected_inventories() {
        let root = fixture();
        let bridge = bridge(&root).await;
        let count = calls(root.path()).len();
        std::fs::write(root.path().join("inventory"), listing).unwrap();
        let rejected = bridge.close().await.is_err();
        let retained = !root.retired() && !empty(root.path());
        let observations = calls(root.path())[count..].to_vec();
        std::fs::remove_file(root.path().join("inventory")).unwrap();
        drop(bridge);
        until(|| root.retired()).await;
        assert!(rejected && retained);
        assert!(observations.iter().all(|(_, args)| args == &["devices"]));
    }
}

#[tokio::test]
async fn t1657_failed_inventory_exit_cannot_authorize_device_queries() {
    let root = fixture();
    let bridge = bridge(&root).await;
    let count = calls(root.path()).len();
    std::fs::write(root.path().join("fail_inventory"), "").unwrap();
    let rejected = bridge.close().await.is_err();
    let retained = !root.retired() && !empty(root.path());
    let observations = calls(root.path())[count..].to_vec();
    std::fs::remove_file(root.path().join("fail_inventory")).unwrap();
    drop(bridge);
    until(|| root.retired()).await;
    assert!(rejected && retained);
    assert!(observations.iter().all(|(_, args)| args == &["devices"]));
}

#[tokio::test]
async fn t1657_inventory_child_holds_original_endpoints_after_caller_loss() {
    let root = fixture();
    let bridge = bridge(&root).await;
    let remote = std::fs::read_to_string(root.path().join("created")).unwrap();
    let port: u16 = remote.strip_prefix("tcp:").unwrap().parse().unwrap();
    std::fs::write(root.path().join("hold_inventory"), "").unwrap();
    let count = calls(root.path()).len();
    drop(bridge);
    until(|| root.path().join("inventory_entered").exists()).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let retained = !root.retired() && !empty(root.path());
    let reserved = std::net::TcpListener::bind(("127.0.0.1", port)).is_err();
    let observations = calls(root.path())[count..].to_vec();
    std::fs::remove_file(root.path().join("hold_inventory")).unwrap();
    until(|| root.retired()).await;
    assert!(retained && reserved);
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].1, ["devices"]);
    drop(std::net::TcpListener::bind(("127.0.0.1", port)).unwrap());
}

#[tokio::test]
async fn t1657_nonready_inventory_and_exact_size_boundary_are_present() {
    for status in ["offline", "unauthorized", "device"] {
        let root = fixture();
        let bridge = bridge(&root).await;
        let mut inventory = format!("List of devices attached\noriginal\t{status}\n");
        inventory.extend(std::iter::repeat_n(' ', 65536 - inventory.len()));
        std::fs::write(root.path().join("inventory"), inventory).unwrap();
        bridge.close().await.unwrap();
        assert!(root.retired());
        assert!(empty(root.path()));
    }
}
