use super::*;
use blent_config::{
    connection::pairing::PeerId,
    tablets::{Identity, TabletSession},
};
use std::os::unix::fs::PermissionsExt;
fn native() -> TabletSession {
    TabletSession {
        identity: Identity::UsbPeer(PeerId::parse("ab".repeat(32)).unwrap()),
        instance: 0,
        video_port: 8890,
        input_port: 8891,
    }
}
#[tokio::test]
async fn t922_doctor_reports_native_display_without_adb() {
    let mut report = Report::new();
    let selected = check_tablet_with(
        &mut report,
        &FileConfig::default(),
        "/missing-t922-adb",
        Some(vec![native()]),
    )
    .await;
    assert!(selected.is_empty(), "native peer became an ADB candidate");
    let messages = report.messages.borrow().join("\n");
    assert!(
        messages.contains("USB tablet (abababababab)"),
        "T922 native status lost without ADB: {messages}"
    );
    assert_eq!((report.warnings, report.failures), (0, 0));
}
#[tokio::test]
async fn t922_native_status_cannot_dispatch_adb_device_commands() {
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$0.log\"\n").unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut report = Report::new();
    check_tablet_session(
        &mut report,
        &FileConfig::default(),
        adb.to_str().unwrap(),
        &native(),
    )
    .await;
    assert!(
        !adb.with_extension("log").exists(),
        "T922 native identity dispatched ADB commands"
    );
}
#[tokio::test]
async fn t922_mixed_status_survives_failed_adb_inventory() {
    let mut adb = native();
    adb.instance = 1;
    adb.identity = Identity::Adb("missing".into());
    let mut report = Report::new();
    let selected = check_tablet_with(
        &mut report,
        &FileConfig::default(),
        "/missing-t922-adb",
        Some(vec![native(), adb]),
    )
    .await;
    assert!(selected.is_empty());
    let messages = report.messages.borrow().join("\n");
    assert!(messages.contains("USB tablet (abababababab)"));
    assert!(messages.contains("no longer reachable"));
    assert_eq!((report.warnings, report.failures), (2, 0));
}
#[tokio::test]
async fn t922_adb_inventory_failures_and_empty_reports_remain_visible() {
    for sessions in [None, Some(vec![])] {
        let mut report = Report::new();
        assert!(check_tablet_with(
            &mut report,
            &FileConfig::default(),
            "/missing-t922-adb",
            sessions
        )
        .await
        .is_empty());
        assert_eq!(report.warnings, 1);
    }
    let mut report = Report::new();
    report_empty_tablets(&mut report, "List of devices attached\nUSB unauthorized\n");
    assert_eq!(report.failures, 1);
    report_empty_tablets(&mut report, "");
    assert_eq!(report.warnings, 1);
    assert_eq!(
        tablet_devices(
            "List of devices attached\n\nshort\nBAD offline\n192.0.2.1:5555 device\nUSB device\n"
        ),
        ["USB", "192.0.2.1:5555"]
    );
}
