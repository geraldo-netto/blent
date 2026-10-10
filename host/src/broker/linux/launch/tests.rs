use super::*;
#[test]
fn t894_paths_are_absolute_and_do_not_probe_devices_or_tokens() {
    let launch = Launch::from_paths(
        "/opt/blent/gui".into(),
        Some("/run/user/42".into()),
        None,
        Some("/home/test".into()),
    )
    .unwrap();
    assert_eq!(
        launch.executable,
        PathBuf::from("/opt/blent/blent-usb-broker")
    );
    assert_eq!(launch.helper, PathBuf::from("/opt/blent/blent-usb-io"));
    assert_eq!(launch.runtime, PathBuf::from("/run/user/42/blent-usb"));
    assert_eq!(
        launch.state,
        PathBuf::from("/home/test/.local/state/blent/connection")
    );
    let launch = Launch::from_paths(
        "/bin/gui".into(),
        Some("/tmp/private".into()),
        Some("/state".into()),
        None,
    )
    .unwrap();
    assert_eq!(launch.state, PathBuf::from("/state/blent/connection"));
    for value in [
        None,
        Some("".into()),
        Some("relative".into()),
        Some("../escape".into()),
    ] {
        assert!(absolute(value.clone(), "test").is_err());
        assert!(
            Launch::from_paths("/bin/gui".into(), value, None, Some("/home/test".into())).is_err()
        );
    }
    assert!(Launch::from_paths("gui".into(), Some("/run".into()), None, None).is_err());
    assert!(Launch::from_paths("/".into(), Some("/run".into()), None, None).is_err());
    assert!(Launch::from_paths(
        "/bin/gui".into(),
        Some("/run".into()),
        Some("relative".into()),
        None
    )
    .is_err());
    assert!(Launch::from_paths("/bin/gui".into(), Some("/run".into()), None, None).is_err());
}

#[test]
fn t924_native_source_start_failure_retires_actual_worker() {
    use crate::{
        broker::linux::presentation::{Factory, Settings},
        display_discovery::Discovery,
    };
    use std::time::{Duration, Instant};
    let root = tempfile::tempdir().unwrap();
    let launch = Launch {
        executable: root.path().join("missing-broker"),
        helper: root.path().join("missing-helper"),
        runtime: root.path().join("runtime"),
        state: root.path().join("state"),
    };
    let settings = Settings::new(launch);
    let factory = Factory::new(settings).unwrap();
    let mut discovery =
        Discovery::new(Box::new(factory), Duration::from_secs(1), Instant::now()).unwrap();
    assert!(discovery.poll(Instant::now()).unwrap().is_empty());
    discovery.close();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !discovery.retired() {
        assert!(
            Instant::now() < deadline,
            "T924 source native worker did not retire"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(discovery.poll(Instant::now()).unwrap().is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn t924_environment_paths_are_lazy_in_isolated_process() {
    use blent_config::commands::SyncCommandExt;
    if std::env::var_os("BLENT_T924_PATHS").is_some() {
        let launch = Launch::from_environment().unwrap();
        assert!(launch.executable.is_absolute());
        assert_eq!(
            launch.state,
            std::path::Path::new(&std::env::var_os("XDG_STATE_HOME").unwrap())
                .join("blent/connection")
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "broker::linux::launch::tests::t924_environment_paths_are_lazy_in_isolated_process",
        ])
        .env("BLENT_T924_PATHS", "1")
        .env("XDG_RUNTIME_DIR", root.path())
        .env("XDG_STATE_HOME", root.path())
        .output_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
