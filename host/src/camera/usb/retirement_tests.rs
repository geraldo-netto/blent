//! T1126: native USB creation must fund/retain its original completion.
use super::*;
use crate::camera_control::{NativeResource, Report};
use futures_util::FutureExt;

fn new_report() -> Report {
    Report::new(
        tokio::sync::watch::channel(blent_config::camera::CameraState::Starting).0,
        tokio::sync::watch::channel(None).0,
    )
}
fn launch(root: &std::path::Path) -> Launch {
    Launch {
        executable: "/bin/false".into(),
        runtime: root.join("runtime"),
        state: root.join("state"),
        helper: root.join("missing-helper"),
    }
}
#[tokio::test]
async fn t1126_usb_creation_requires_original_observation_before_native_setup() {
    let root = tempfile::tempdir().unwrap();
    let report = new_report();
    let reserved = report.reserve_native(NativeResource::Connection).unwrap();
    let called = std::cell::Cell::new(false);
    let result = native(
        &crate::camera_registration::test_support::profile(),
        &"ab".repeat(32),
        || {
            called.set(true);
            TcpListener::bind("127.0.0.1:0")
        },
        launch(root.path()),
        &report,
    );
    let rejected = result.is_err();
    let created = called.get();
    drop(result);
    drop(reserved);
    report.wait_native().await;
    assert!(
        !created,
        "T1126 listener creator ran before original admission"
    );
    assert!(
        rejected,
        "T1126 USB constructor ignored original observation exhaustion"
    );
}
#[tokio::test]
async fn t1126_usb_original_bridge_drop_not_logical_construction_settles_proof() {
    let root = tempfile::tempdir().unwrap();
    let report = new_report();
    let bridge = native(
        &crate::camera_registration::test_support::profile(),
        &"ab".repeat(32),
        || TcpListener::bind("127.0.0.1:0"),
        launch(root.path()),
        &report,
    )
    .unwrap();
    let retained = report.wait_native().now_or_never().is_none();
    drop(bridge);
    tokio::time::timeout(std::time::Duration::from_secs(3), report.wait_native())
        .await
        .unwrap();
    assert!(
        retained,
        "T1126 USB native original completion was not retained"
    );
}

#[tokio::test]
async fn t1126_failed_listener_creator_releases_unbound_original_observation() {
    let root = tempfile::tempdir().unwrap();
    let report = new_report();
    let failed = native(
        &crate::camera_registration::test_support::profile(),
        &"ab".repeat(32),
        || Err(std::io::Error::other("T1126 listener creation failed")),
        launch(root.path()),
        &report,
    );
    assert!(failed.is_err());
    assert!(report.wait_native().now_or_never().is_some());
    drop(report.reserve_native(NativeResource::Connection).unwrap());
}

#[tokio::test]
async fn t1126_invalid_original_credential_never_invokes_listener_creator() {
    let root = tempfile::tempdir().unwrap();
    let mut values = vec![
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "G".repeat(64),
        "\0".repeat(64),
        "é".repeat(32),
    ];
    values.extend((0..32).map(|seed| char::from_u32(0x100 + seed).unwrap().to_string().repeat(32)));
    for token in values {
        let report = new_report();
        let called = std::cell::Cell::new(false);
        let result = native(
            &crate::camera_registration::test_support::profile(),
            &token,
            || {
                called.set(true);
                TcpListener::bind("127.0.0.1:0")
            },
            launch(root.path()),
            &report,
        );
        assert!(result.is_err());
        assert!(
            !called.get(),
            "T1126 invalid credential reached native effects"
        );
        assert!(report.wait_native().now_or_never().is_some());
    }
}
