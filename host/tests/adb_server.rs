//! T1465: the installed entry point bootstraps the same PATH-selected ADB.
#![cfg(target_os = "linux")]
use std::{os::unix::fs::PermissionsExt, process::Command};

#[test]
fn t1465_bootstrap_dispatch_waits_and_preserves_arguments() {
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(
        &adb,
        "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit \"$BLENT_TEST_STATUS\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    for code in [0, 1] {
        let output = Command::new(env!("CARGO_BIN_EXE_blent"))
            .arg("--blent-start-adb-server")
            .env("PATH", root.path())
            .env("BLENT_TEST_STATUS", code.to_string())
            .env("ADB_SERVER_SOCKET", "tcp:127.0.0.1:5037")
            .output()
            .unwrap();
        assert_eq!(output.status.success(), code == 0);
        assert_eq!(output.stdout, b"-L\ntcp:5037\nstart-server\n");
    }
}
