use super::*;

#[tokio::test]
async fn t1378_failed_identity_revokes_session_but_preserves_original_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    let (adb, monitor, _stop, _ports) = monitor_fixture().await;
    let mut monitor = monitor.with_network(store);
    adb.0
         .0
        .lock()
        .unwrap()
        .identities
        .insert("USB".into(), "original".into());
    monitor.poll().await;
    assert_eq!(monitor.sessions().len(), 1);
    let owned = adb.0 .0.lock().unwrap().routes.clone();
    adb.0 .0.lock().unwrap().fail = Some("getprop".into());
    monitor.poll().await;
    assert!(
        monitor.sessions().is_empty(),
        "T1378: failed identity kept old session"
    );
    assert!(
        monitor.shutdown().await.is_err(),
        "T1378: unknown cleanup was discarded"
    );
    assert_eq!(adb.0 .0.lock().unwrap().routes, owned);
    {
        let mut state = adb.0 .0.lock().unwrap();
        state.fail = None;
        state.identities.insert("USB".into(), "replacement".into());
    }
    monitor.poll().await;
    assert!(monitor.sessions().is_empty());
    assert_eq!(adb.0 .0.lock().unwrap().routes, owned);
    adb.0
         .0
        .lock()
        .unwrap()
        .identities
        .insert("USB".into(), "original".into());
    monitor.poll().await;
    assert_eq!(monitor.sessions().len(), 1);
    monitor.shutdown().await.unwrap();
    assert!(adb.0 .0.lock().unwrap().routes.is_empty());
}

#[tokio::test]
async fn t1378_invalid_identity_never_reuses_cached_retirement_authority() {
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    let mut network = network::Network::new(store);
    let (adb, _) = fixture();
    adb.0 .0.lock().unwrap().inventory = "List of devices attached\nUSB\tdevice\n".into();
    for invalid in ["".into(), "unknown".into(), "a\0b".into(), "x".repeat(1025)] {
        adb.0
             .0
            .lock()
            .unwrap()
            .identities
            .insert("USB".into(), "original".into());
        network.devices(&adb, &[]).await.unwrap();
        let original = network.identity("USB").cloned().unwrap();
        adb.0
             .0
            .lock()
            .unwrap()
            .identities
            .insert("USB".into(), invalid);
        network.devices(&adb, &["USB".into()]).await.unwrap();
        assert_eq!(
            network.identity("USB"),
            None,
            "T1378: invalid observation retained identity"
        );
        let mut routes = Routes::new("USB", (9000, 9001)).unwrap();
        assert!(!network.can_retire(&adb, &mut routes, Some(&original)).await);
    }
}
