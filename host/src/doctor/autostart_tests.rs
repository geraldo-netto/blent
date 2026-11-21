use super::*;
use std::os::unix::fs::PermissionsExt;

struct Case {
    test: &'static str,
    state: &'static str,
    exit: &'static str,
    load: &'static str,
    desktop: bool,
    expected: &'static str,
    forbidden: &'static str,
}

async fn observe(case: Case) {
    if std::env::var_os("BLENT_T1515_CHILD").is_some() {
        let mut report = Report::new();
        check_autostart(&mut report).await;
        let text = report.messages.borrow().join("\n");
        assert!(
            text.contains(case.expected),
            "T1515 missing observation: {text}"
        );
        assert!(
            !text.contains(case.forbidden),
            "T1515 incorrect observation: {text}"
        );
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("systemctl");
    std::fs::write(&program, "#!/bin/sh\ncase \"$*\" in\n  '--user is-enabled blent.service') printf '%s\\n' \"$BLENT_T1515_STATE\"; exit \"$BLENT_T1515_EXIT\";;\n  '--user show blent.service -p LoadState --value') printf '%s\\n' \"$BLENT_T1515_LOAD\";;\nesac\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = root.path().join("config");
    std::fs::create_dir_all(config.join("autostart")).unwrap();
    if case.desktop {
        std::fs::write(
            config.join("autostart/blent.desktop"),
            "[Desktop Entry]\nType=Application\nExec=/private/blent start\n",
        )
        .unwrap();
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("doctor::autostart_tests::{}", case.test),
            "--nocapture",
        ])
        .env("BLENT_T1515_CHILD", "1")
        .env("BLENT_T1515_STATE", case.state)
        .env("BLENT_T1515_EXIT", case.exit)
        .env("BLENT_T1515_LOAD", case.load)
        .env("XDG_CONFIG_HOME", config)
        .env("PATH", root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn t1515_disabled_exit_retains_enable_hint() {
    observe(Case {
        test: "t1515_disabled_exit_retains_enable_hint",
        state: "disabled",
        exit: "1",
        load: "loaded",
        desktop: false,
        expected: "systemctl --user enable --now blent.service",
        forbidden: "service not installed",
    })
    .await;
}

#[tokio::test]
async fn t1515_static_unit_is_not_reported_as_disabled() {
    observe(Case {
        test: "t1515_static_unit_is_not_reported_as_disabled",
        state: "static",
        exit: "0",
        load: "loaded",
        desktop: false,
        expected: "static",
        forbidden: "disabled",
    })
    .await;
}

#[tokio::test]
async fn t1515_runtime_enablement_is_named_explicitly() {
    observe(Case {
        test: "t1515_runtime_enablement_is_named_explicitly",
        state: "enabled-runtime",
        exit: "0",
        load: "loaded",
        desktop: false,
        expected: "enabled-runtime",
        forbidden: "disabled",
    })
    .await;
}

#[tokio::test]
async fn t1515_desktop_entry_does_not_require_a_service() {
    observe(Case {
        test: "t1515_desktop_entry_does_not_require_a_service",
        state: "",
        exit: "1",
        load: "not-found",
        desktop: true,
        expected: "desktop entry",
        forbidden: "service not installed",
    })
    .await;
}

#[tokio::test]
async fn t1515_masked_nonzero_state_remains_visible() {
    observe(Case {
        test: "t1515_masked_nonzero_state_remains_visible",
        state: "masked",
        exit: "1",
        load: "loaded",
        desktop: false,
        expected: "systemd state: masked",
        forbidden: "disabled",
    })
    .await;
}

#[tokio::test]
async fn t1515_invalid_command_reply_is_unavailable() {
    observe(Case {
        test: "t1515_invalid_command_reply_is_unavailable",
        state: "not a state",
        exit: "1",
        load: "loaded",
        desktop: false,
        expected: "could not query systemd autostart",
        forbidden: "disabled",
    })
    .await;
}

#[test]
fn t1515_state_text_is_bounded_and_rejects_control_or_invalid_bytes() {
    for state in ["enabled", "disabled", "enabled-runtime", "static", "masked"] {
        assert_eq!(
            autostart::state_text(format!(" {state}\n").as_bytes()).as_deref(),
            Some(state)
        );
    }
    assert_eq!(autostart::state_text(&vec![b'a'; 64]).unwrap().len(), 64);
    for bytes in [
        vec![],
        vec![b'a'; 65],
        vec![0xff],
        b"Enabled".to_vec(),
        b"two states".to_vec(),
    ] {
        assert!(autostart::state_text(&bytes).is_none());
    }
    for invalid in 0u8..=255 {
        if invalid.is_ascii_lowercase() || invalid == b'-' {
            continue;
        }
        assert!(
            autostart::state_text(&[b'a', invalid, b'b']).is_none(),
            "{invalid}"
        );
    }
}

#[test]
fn t1515_desktop_observation_distinguishes_absence_disabled_and_unavailable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("entry");
    assert_eq!(autostart::desktop_at(&path).unwrap(), false);
    for (text, enabled) in [
        ("", false),
        ("[Other]\nType=Application\nExec=blent\n", false),
        (
            "[Desktop Entry]\nType=Application\nExec=blent\nHidden=true\n",
            false,
        ),
        (
            "[Desktop Entry]\nType=Application\nExec=blent\nHidden=false\n",
            true,
        ),
    ] {
        std::fs::write(&path, text).unwrap();
        assert_eq!(autostart::desktop_at(&path).unwrap(), enabled);
    }
    std::fs::write(&path, [0xff]).unwrap();
    assert!(autostart::desktop_at(&path).is_err());
    assert!(autostart::desktop_at(&path.join("nested")).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(autostart::desktop_at(&path).is_err());
    let mut report = Report::new();
    report_autostart(&mut report, "loaded", None, None);
    assert_eq!(report.warnings, 2);
}

#[test]
fn t1515_desktop_capacity_and_fifo_are_rejected_without_unbounded_reading() {
    use std::os::unix::ffi::OsStrExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("entry");
    let mut text = b"[Desktop Entry]\nType=Application\nExec=blent\n#".to_vec();
    text.resize(64 * 1024, b'a');
    std::fs::write(&path, &text).unwrap();
    assert!(autostart::desktop_at(&path).unwrap());
    text.push(b'a');
    std::fs::write(&path, &text).unwrap();
    assert!(autostart::desktop_at(&path).is_err());
    std::fs::remove_file(&path).unwrap();
    let native = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    assert!(autostart::desktop_at(&path).is_err());
}
