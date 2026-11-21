//! T1599: exercise the shipped broker entry with private state and a failing
//! helper. No physical USB device or installed broker is touched.
#![cfg(all(target_os = "linux", feature = "accessory-transport"))]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_diagnostic(root: &std::path::Path, child: &mut OwnedChild) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let output = fs::read_to_string(root.join("stdout")).unwrap();
        let error = fs::read_to_string(root.join("stderr")).unwrap();
        if output.contains("USB inventory unavailable")
            || error.contains("USB inventory unavailable")
        {
            return;
        }
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "broker exited before inventory warning: {error}"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "fixture produced no warning: {error}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn t1599_on_demand_broker_preserves_warnings_on_standard_error() {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("helper");
    fs::write(&helper, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = OwnedChild(
        Command::new(env!("CARGO_BIN_EXE_blent-usb-broker"))
            .arg("--runtime")
            .arg(root.path().join("runtime"))
            .arg("--state")
            .arg(root.path().join("state"))
            .arg("--helper")
            .arg(helper)
            .stdin(Stdio::null())
            .stdout(fs::File::create(root.path().join("stdout")).unwrap())
            .stderr(fs::File::create(root.path().join("stderr")).unwrap())
            .spawn()
            .unwrap(),
    );
    wait_for_diagnostic(root.path(), &mut child);
    // Only this exact child PID is signalled, then reaped before its private files.
    assert_eq!(unsafe { libc::kill(child.0.id() as i32, libc::SIGTERM) }, 0);
    assert!(child.0.wait().unwrap().success());
    let error = fs::read_to_string(root.path().join("stderr")).unwrap();
    assert!(
        error.contains("USB inventory unavailable"),
        "T1599 warnings were discarded with stdout"
    );
    assert!(fs::read(root.path().join("stdout")).unwrap().is_empty());
}
