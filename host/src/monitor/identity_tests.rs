use super::*;
use std::os::unix::fs::PermissionsExt;

fn identity_adb(root: &std::path::Path) -> PathBuf {
    let adb = root.join("adb");
    std::fs::write(
        &adb,
        r#"#!/bin/sh
if [ "$4" = getprop ]; then cat "$0.$2"; exit; fi
if [ "$4" = pm ]; then echo package:/app/blent.apk; fi
"#,
    )
    .unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(adb.with_extension("OLD"), "original").unwrap();
    std::fs::write(adb.with_extension("NEW"), "unrelated").unwrap();
    adb
}

async fn observe(state: &mut Monitor) {
    state
        .discovery
        .refresh(vec!["OLD".into(), "NEW".into()], &state.config.adb);
    for _ in 0..2 {
        let (serial, identity) = state.discovery.next().await.unwrap();
        state.probe_ready(serial, identity);
    }
}

#[tokio::test]
async fn t1380_same_endpoint_replacement_retires_only_original_attachment() {
    let root = tempfile::tempdir().unwrap();
    let adb = identity_adb(root.path());
    let mut state = tests::fixture();
    state.config.adb = adb.to_str().unwrap().into();
    observe(&mut state).await;
    state.current = Some("OLD".into());
    state.config.tablet.begin(Some("device:original".into()));
    state
        .attachments
        .insert("OLD".into(), state.config.tablet.clone());
    state.ready.insert("OLD".into());
    state
        .slots
        .reserve(DisplayOwner::Adb("OLD".into()), 0)
        .unwrap();
    let accepted = state.config.tablet.lease();
    let unrelated = state.launches.observe("NEW", Some("unrelated")).unwrap();
    assert!(unrelated.take());
    std::fs::write(adb.with_extension("OLD"), "replacement").unwrap();
    observe(&mut state).await;
    let stale_accepted = accepted.apply(|| {});
    let identity = state.identities.get("OLD").cloned();
    let removed = !state.attachments.contains_key("OLD");
    let other = state.identities.get("NEW").cloned();
    let repeated_launch = unrelated.take();
    state.stop().await;
    assert_eq!(
        identity.as_deref(),
        Some("replacement"),
        "T1380: cached identity hid replacement"
    );
    assert!(
        !stale_accepted,
        "T1380: replaced tablet retained attachment authority"
    );
    assert!(removed);
    assert_eq!(other.as_deref(), Some("unrelated"));
    assert!(!repeated_launch);
}

#[tokio::test]
async fn t1380_failed_revalidation_reports_unknown_then_recovers() {
    let root = tempfile::tempdir().unwrap();
    let adb = identity_adb(root.path());
    let mut discovery = discovery::Discovery::new();
    for expected in [Some("original"), None, Some("replacement")] {
        std::fs::write(adb.with_extension("OLD"), expected.unwrap_or("unknown")).unwrap();
        discovery.refresh(vec!["OLD".into()], adb.to_str().unwrap());
        let (_, identity) = discovery.next().await.unwrap();
        assert_eq!(
            identity.as_deref(),
            expected,
            "T1380: observation was not refreshed"
        );
    }
    discovery.stop().await;
}
