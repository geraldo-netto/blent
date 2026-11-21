//! T497/T1640: query CLI adapters and native writes share a private bus fixture.
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const TEST: &str = "kwin::coverage_tests::t497_kwin_adapters_restore_only_the_owned_keyboard_state";

const COMMAND: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$HOME/calls"
case "$*" in
  *available*) printf 'b true\n';;
  *mode*)
    read -r value < "$HOME/mode"
    if [ "$BLENT_T497_BACKEND" = busctl ]; then printf 'i %s\n' "$value"
    else printf '[Variant(int): %s]\n' "$value"; fi;;
  *bad*) exit 42;;
  *) /bin/cat "$HOME/reply";;
esac
"#;

fn isolated(backend: &str) {
    let directory = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let tool = directory.path().join(backend);
    std::fs::write(&tool, COMMAND).unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(directory.path().join("mode"), "2\n").unwrap();
    let output = std::process::Command::new("/usr/bin/dbus-run-session")
        .args(["--dbus-daemon=/usr/bin/dbus-daemon", "--"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env("BLENT_T497_BACKEND", backend)
        .env("PATH", directory.path())
        .env("HOME", directory.path())
        .env("XDG_RUNTIME_DIR", directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(directory.path().join("calls")).unwrap();
    assert!(calls.contains("mode"));
    assert!(calls.contains("fixtureDevices"));
}

struct NativeKeyboard(std::path::PathBuf);
#[zbus::interface(name = "org.kde.kwin.VirtualKeyboard")]
impl NativeKeyboard {
    #[zbus(property, name = "mode")]
    fn mode(&self) -> i32 {
        std::fs::read_to_string(&self.0)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }
    #[zbus(property, name = "mode")]
    fn set_mode(&mut self, mode: i32) {
        std::fs::write(&self.0, format!("{mode}\n")).unwrap();
    }
}
async fn native_keyboard(root: &Path) -> zbus::Connection {
    zbus::connection::Builder::session()
        .unwrap()
        .name("org.kde.KWin")
        .unwrap()
        .serve_at("/VirtualKeyboard", NativeKeyboard(root.join("mode")))
        .unwrap()
        .build()
        .await
        .unwrap()
}

async fn properties(root: &Path, busctl: bool) {
    let response = root.join("reply");
    let text = if busctl {
        "s \"DVI-I-1\""
    } else {
        "[Variant(QString): \"DVI-I-1\"]"
    };
    std::fs::write(&response, text).unwrap();
    assert_eq!(
        get_property("/fixture", "fixture", "name").await.as_deref(),
        Some("DVI-I-1")
    );
    let text = if busctl {
        "as 2 \"event7\" \"event8\""
    } else {
        "event7\nevent8\n"
    };
    std::fs::write(&response, text).unwrap();
    assert_eq!(
        list_strings("/fixture", "fixture", "fixtureDevices")
            .await
            .unwrap(),
        ["event7", "event8"]
    );
    std::fs::write(&response, "").unwrap();
    assert!(list_strings("/fixture", "fixture", "fixtureDevices")
        .await
        .unwrap()
        .is_empty());
    assert!(get_property("/bad", "fixture", "name").await.is_none());
    assert!(list_strings("/bad", "fixture", "fixtureDevices")
        .await
        .is_none());
}

async fn keyboard(root: &Path) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let devices = AtomicUsize::new(1);
    crate::osk::sync_touch_state(&devices, &|_| Ok(())).await;
    let backup = root.join(".local/share/blent/osk-restore");
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), "2");
    assert_eq!(std::fs::read_to_string(root.join("mode")).unwrap(), "0\n");
    devices.store(0, Ordering::SeqCst);
    crate::osk::sync_touch_state(&devices, &|_| Ok(())).await;
    assert_eq!(std::fs::read_to_string(root.join("mode")).unwrap(), "2\n");
    assert!(!backup.exists());
    crate::osk::Restoration::new().unwrap().run().await;
    assert!(!backup.exists());
}

#[tokio::test]
async fn t497_kwin_adapters_restore_only_the_owned_keyboard_state() {
    let Ok(kind) = std::env::var("BLENT_T497_BACKEND") else {
        for backend in ["busctl", "qdbus", "qdbus6", "qdbus-qt6", "qdbus-qt5"] {
            isolated(backend);
        }
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
    assert_eq!(backend().await.unwrap().name(), kind);
    let native = native_keyboard(&root).await;
    properties(&root, kind == "busctl").await;
    keyboard(&root).await;
    let commands = crate::native_commands::ObservedCommands::default();
    assert!(
        set_property_with(
            &commands,
            "/VirtualKeyboard",
            "org.kde.kwin.VirtualKeyboard",
            "mode",
            "i",
            "1"
        )
        .await
    );
    while !commands.complete() {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        get_property("/fixture", "fixture", "mode").await.as_deref(),
        Some("1")
    );
    native.close().await.unwrap();
}
