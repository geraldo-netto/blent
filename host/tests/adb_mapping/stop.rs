//! T1204: exercise actual native probe commands with a private local fixture.
#![cfg(target_os = "linux")]
use super::{adb_command, bridge};
#[path = "../../examples/camera-feedback-probe/stop.rs"]
mod stop;

#[tokio::test]
async fn t1204_probe_stop_is_a_scoped_command_not_a_replacement_invitation() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, "#!/usr/bin/python3\nimport pathlib,sys\npathlib.Path(__file__).with_suffix('.input').write_text(sys.stdin.read())\nprint('Broadcast completed: result=1')\n").unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = bridge::Target {
        adb,
        serial: "original-tablet".into(),
        remote: "tcp:34567".into(),
        local: "tcp:12345".into(),
    };
    let credential = "ab".repeat(32);
    stop::stop_capture(&bridge, &credential).await.unwrap();
    let command = std::fs::read_to_string(root.path().join("adb.input")).unwrap();
    assert!(
        command.contains(" -a com.blent.CAMERA_STOP "),
        "T1204 unscoped replacement invitation: {command}"
    );
    assert!(command.contains("--ei expected_port 34567"));
    assert!(!command.contains("--ei width"));
    assert!(command.contains(&format!("--es expected_token {credential}")));
}

#[test]
fn t1204_original_adb_port_rejects_invalid_and_out_of_range_endpoints() {
    let mut bridge = bridge::Target {
        adb: "not-invoked".into(),
        serial: "original".into(),
        remote: String::new(),
        local: "tcp:34567".into(),
    };
    for remote in [
        "",
        "tcp:",
        "tcp:0",
        "tcp:-1",
        "tcp:65536",
        "tcp:4294967295",
        "udp:1",
        "tcp:1;id",
    ] {
        bridge.remote = remote.into();
        assert!(bridge.port().is_err(), "{remote}");
    }
    for port in [1, 1024, 65535] {
        bridge.remote = format!("tcp:{port}");
        assert_eq!(bridge.port().unwrap(), port);
    }
}

#[tokio::test]
async fn t1204_native_stop_requires_exact_acknowledgement_and_command_success() {
    let root = tempfile::tempdir().unwrap();
    for (stdout, expected) in adb_command::broadcast_tests::cases() {
        let bridge = bridge::Target {
            adb: adb_command::broadcast_tests::fixture(root.path(), &stdout, true),
            serial: "original".into(),
            remote: "tcp:34567".into(),
            local: "tcp:12345".into(),
        };
        assert_eq!(
            stop::stop_capture(&bridge, &"ab".repeat(32)).await.is_ok(),
            expected
        );
    }
    let bridge = bridge::Target {
        adb: adb_command::broadcast_tests::fixture(
            root.path(),
            b"Broadcast completed: result=1\n",
            false,
        ),
        serial: "original".into(),
        remote: "tcp:34567".into(),
        local: "tcp:12345".into(),
    };
    assert!(stop::stop_capture(&bridge, &"ab".repeat(32)).await.is_err());
}

#[tokio::test]
async fn t1204_invalid_stop_credential_rejects_before_native_command_creation() {
    let root = tempfile::tempdir().unwrap();
    let bridge = bridge::Target {
        adb: root.path().join("absent-adb"),
        serial: "original".into(),
        remote: "tcp:34567".into(),
        local: "tcp:12345".into(),
    };
    for invalid in ["", "short", "$(touch ignored)", "é"] {
        assert!(stop::stop_capture(&bridge, invalid)
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid camera token"));
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
