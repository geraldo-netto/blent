use super::*;

#[tokio::test]
async fn t1274_configured_ipc_capacity_rejects_invalid_native_input() {
    if let Some(value) = std::env::var_os("BLENT_T1274_CAPACITY") {
        let result = Control::bind();
        assert_eq!(result.is_ok(), value == "valid");
        if let Ok(control) = result {
            control.close().await;
        }
        return;
    }
    use std::os::unix::ffi::OsStringExt;
    for (value, expected) in [
        (std::ffi::OsString::from("2"), "valid"),
        (std::ffi::OsString::from("0"), "invalid"),
        (std::ffi::OsString::from_vec(vec![255]), "invalid"),
    ] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "monitor::control::tests::t1274_configured_ipc_capacity_rejects_invalid_native_input"])
            .env("BLENT_T1274_CAPACITY", expected).env("BLENT_DISPLAY_CONTROL_CLIENTS", value).output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn t1274_display_ipc_capacity_is_user_bounded() {
    for value in ["", "0", "65", "-1", "invalid", "999999999999999999999999"] {
        assert!(parse_capacity(value).is_err());
    }
    for value in 1..=64 {
        assert_eq!(parse_capacity(&value.to_string()).unwrap(), value);
    }
}

#[tokio::test]
async fn t1274_native_daemon_control_round_trip_and_caller_loss() {
    assert!(
        tokio::time::timeout(Duration::from_millis(1), event(&mut None))
            .await
            .is_err()
    );
    let mut state = crate::monitor::tests::fixture();
    state.control = Some(Control::bind().unwrap());
    assert!(Control::bind().is_err());
    let client = tokio::task::spawn_blocking(|| {
        let client = blent_config::linux::display_control::Client::new().unwrap();
        let reply = client
            .request(std::process::id(), &Command::Inspect {})
            .unwrap();
        assert!(reply.error.is_none());
        let command = Command::Set {
            owner: reply.snapshot.owner,
            target: blent_config::display_control::Target::Adb("offline-tablet".into()),
            enabled: true,
            expected: None,
        };
        client.request(std::process::id(), &command).unwrap()
    });
    for _ in 0..2 {
        let call = event(&mut state.control).await;
        apply_event(&mut state, Event::Control(call)).await;
    }
    // The next event polls the response writer; cancelling this read leaves
    // the writer in its original bounded task owner.
    assert!(
        tokio::time::timeout(Duration::from_millis(100), event(&mut state.control))
            .await
            .is_err()
    );
    assert!(client.await.unwrap().error.is_none());
    apply_event(
        &mut state,
        Event::Control(Err(anyhow::anyhow!("transport exhausted"))),
    )
    .await;
    apply_event(&mut state, Event::ExtraToken("absent".into())).await;
    apply_event(&mut state, Event::Probe(None)).await;
    state.publish(&mut None);
    assert!(throttle(Err(anyhow::anyhow!("accept exhausted")))
        .await
        .is_err());
    let root = tempfile::tempdir().unwrap();
    let mut ledger = Some(runtime::SessionLedger::new(root.path().join("sessions.json")).unwrap());
    root.close().unwrap();
    state.current = Some("ledger-fault".into());
    state.ready.insert("ledger-fault".into());
    state.publish(&mut ledger);
    state.current = None;
    state.ready.clear();
    caller_lost(&mut state).await;
    state.stop().await;
}

async fn caller_lost(state: &mut Monitor) {
    use std::{io::Write, os::linux::net::SocketAddrExt};
    let ticket = state
        .slots
        .reserve(DisplayOwner::Adb("caller-lost".into()), 0)
        .unwrap();
    state.current = Some("caller-lost".into());
    state.ready.insert("caller-lost".into());
    let owner = state.control.as_ref().unwrap().owner.clone();
    let command = Command::Set {
        owner,
        target: blent_config::display_control::Target::Adb("caller-lost".into()),
        enabled: false,
        expected: Some(ticket.lease()),
    };
    let pid = std::process::id();
    let start = blent_config::linux::processes::start_ticks(pid).unwrap();
    let name = format!("blent-display-{}-{pid}-{start}", unsafe { libc::getuid() });
    let address = std::os::unix::net::SocketAddr::from_abstract_name(name).unwrap();
    let mut client = std::os::unix::net::UnixStream::connect_addr(&address).unwrap();
    client
        .write_all(&serde_json::to_vec(&command).unwrap())
        .unwrap();
    client.shutdown(std::net::Shutdown::Write).unwrap();
    drop(client);
    let call = event(&mut state.control).await;
    apply_event(state, Event::Control(call)).await;
    assert_eq!(state.slots.closing(ticket), Some(true));
    assert!(!state
        .slots
        .enabled(&DisplayOwner::Adb("caller-lost".into())));
    let retired = state.retiring.join_next().await.unwrap().unwrap();
    state.retired(retired);
    assert!(state
        .slots
        .ticket(&DisplayOwner::Adb("caller-lost".into()))
        .is_none());
}

#[tokio::test]
async fn t1274_control_rejects_foreign_owner_and_stale_replacement() {
    let mut state = crate::monitor::tests::fixture();
    let owner = "a".repeat(64);
    let target = blent_config::display_control::Target::Adb("first".into());
    let ticket = state
        .slots
        .reserve(DisplayOwner::Adb("first".into()), 0)
        .unwrap();
    let other = state
        .slots
        .reserve(DisplayOwner::Adb("other".into()), 1)
        .unwrap();
    let mut stop = Command::Set {
        owner: "b".repeat(64),
        target: target.clone(),
        enabled: false,
        expected: Some(ticket.lease()),
    };
    assert!(apply(&mut state, &owner, &stop).is_err());
    if let Command::Set {
        owner: expected, ..
    } = &mut stop
    {
        *expected = owner.clone();
    }
    apply(&mut state, &owner, &stop).unwrap();
    assert_eq!(state.slots.closing(ticket), Some(true));
    assert_eq!(state.slots.closing(other), Some(false));
    assert_eq!(
        snapshot(&state, &owner).assignments[0].phase,
        Phase::Stopping
    );
    let start = Command::Set {
        owner: owner.clone(),
        target: target.clone(),
        enabled: true,
        expected: None,
    };
    assert!(apply(&mut state, &owner, &start).is_err());
    state.slots.retired(ticket).unwrap();
    apply(&mut state, &owner, &start).unwrap();
    let replacement = state
        .slots
        .reserve(DisplayOwner::Adb("first".into()), 0)
        .unwrap();
    assert!(apply(&mut state, &owner, &stop).is_err());
    assert_eq!(state.slots.closing(replacement), Some(false));
    assert_eq!(state.slots.closing(other), Some(false));
    apply(&mut state, &owner, &Command::Inspect {}).unwrap();
    let invalid = Command::Set {
        owner: owner.clone(),
        target,
        enabled: false,
        expected: None,
    };
    assert!(apply(&mut state, &owner, &invalid).is_err());
    state.slots.close(replacement).unwrap();
    state.slots.retired(replacement).unwrap();
    assert!(apply(&mut state, &owner, &stop).is_err());
    state.slots.close(other).unwrap();
    state.slots.retired(other).unwrap();
    state.stop().await;
}

#[tokio::test]
async fn t1274_primary_stop_keeps_other_tablet_and_lease() {
    let mut state = crate::monitor::tests::fixture();
    let owner = "c".repeat(64);
    state.change_primary(&Some("first".into()));
    let ticket = state
        .slots
        .reserve(DisplayOwner::Adb("first".into()), 0)
        .unwrap();
    state.ready.insert("first".into());
    let other = state
        .slots
        .reserve(DisplayOwner::Adb("other".into()), 1)
        .unwrap();
    let before = snapshot(&state, &owner);
    assert_eq!(before.assignments[0].phase, Phase::Assigned);
    apply(
        &mut state,
        &owner,
        &Command::Set {
            owner: owner.clone(),
            target: before.assignments[0].target.clone(),
            enabled: false,
            expected: Some(ticket.lease()),
        },
    )
    .unwrap();
    assert!(state.current.is_none());
    assert_eq!(state.slots.closing(other), Some(false));
    let retired = state.retiring.join_next().await.unwrap().unwrap();
    state.retired(retired);
    assert!(state
        .slots
        .ticket(&DisplayOwner::Adb("first".into()))
        .is_none());
    assert!(!state.slots.enabled(&DisplayOwner::Adb("first".into())));
    state.slots.close(other).unwrap();
    state.slots.retired(other).unwrap();
    state.stop().await;
}
