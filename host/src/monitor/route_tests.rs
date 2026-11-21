//! T705: route ownership across Linux preparation, repair and retirement.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn private(test: &str) -> bool {
    if std::env::var_os("BLENT_T1425_PRIVATE_RUNTIME").is_some() {
        return true;
    }
    successful(isolated(test, "BLENT_T1425_PRIVATE_RUNTIME"));
    false
}

fn isolated(test: &str, marker: &str) -> std::process::Output {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env(marker, "1")
        .env("XDG_RUNTIME_DIR", root.path())
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path())
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/missing-t1425-bus")
        .output()
        .unwrap()
}

fn successful(result: std::process::Output) {
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn t1425_route_fixture_ignores_an_owned_foreign_runtime_namespace() {
    use blent::usb::{Commands, NativeCommands};
    const TEST: &str =
        "monitor::route_tests::t1425_route_fixture_ignores_an_owned_foreign_runtime_namespace";
    if std::env::var_os("BLENT_T1425_LOCK_HOLDER").is_none() {
        successful(isolated(TEST, "BLENT_T1425_LOCK_HOLDER"));
        return;
    }
    // Hold the actual native namespace, using the public lease contract rather
    // than duplicating its stripe algorithm. Every path is this child's own.
    let _held = NativeCommands("unused".into())
        .display_route_lease("USB", (18000, 18001))
        .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "monitor::route_tests::t1346_failed_removals_keep_ticket_until_original_effects_arrive",
            "--nocapture",
        ])
        .env_remove("BLENT_T1425_PRIVATE_RUNTIME")
        .output()
        .unwrap();
    successful(result);
}

fn fixture(listing: &str) -> (tempfile::TempDir, Monitor, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(adb.with_extension("listing"), listing).unwrap();
    std::fs::write(
        &adb,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$0.log"
if [ "$1" = devices ]; then
    printf 'List of devices attached\n'
    [ -f "$0.offline" ] || printf 'USB\tdevice\n'
    exit 0
fi
[ "$3" = reverse ] || { cat >/dev/null; exit 0; }
case "$4" in
--list) [ ! -f "$0.offline" ] || exit 1; cat "$0.listing"; exit 0 ;;
--remove) [ ! -f "$0.fail-remove" ] || exit 1; remote=$5 ;;
--no-rebind) remote=$5; target=$6
    grep -q "^UsbFfs $remote " "$0.listing" && exit 1 ;;
*) remote=$4; target=$5 ;;
esac
[ "$remote" != tcp:8891 ] || [ ! -f "$0.fail-input" ] || exit 1
awk -v remote="$remote" '$2 != remote' "$0.listing" > "$0.next"
mv "$0.next" "$0.listing"
[ "$4" = --remove ] || printf 'UsbFfs %s %s\n' "$remote" "$target" >> "$0.listing"
exit 0
"#,
    )
    .unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut state = tests::fixture();
    state.config.adb = adb.to_str().unwrap().into();
    state.current = Some("USB".into());
    (root, state, adb)
}

async fn prepare(state: &mut Monitor) {
    state.prepare_primary();
    let (serial, result) = state.mutations.next().await;
    state.mutation_ready(serial, result);
}

fn listing(adb: &std::path::Path) -> String {
    std::fs::read_to_string(adb.with_extension("listing")).unwrap()
}

#[tokio::test]
async fn t705_conflicting_initial_routes_remain_foreign() {
    if !private("monitor::route_tests::t705_conflicting_initial_routes_remain_foreign") {
        return;
    }
    let foreign = "UsbFfs tcp:8891 tcp:2222\n";
    let (_root, mut state, adb) = fixture(foreign);
    prepare(&mut state).await;
    let ready = state.ready.contains("USB");
    state.stop().await;
    assert!(!ready, "T705: conflicting initial route was rebound");
    assert_eq!(listing(&adb), foreign);
}

#[tokio::test]
async fn t705_matching_external_and_replaced_routes_survive_retirement() {
    if !private(
        "monitor::route_tests::t705_matching_external_and_replaced_routes_survive_retirement",
    ) {
        return;
    }
    let external = "UsbFfs tcp:8890 tcp:18000\n";
    let (_root, mut state, adb) = fixture(external);
    prepare(&mut state).await;
    let ready = state.ready.contains("USB");
    // T1425/T1345: external owners may replace their own preexisting mapping,
    // never the input endpoint admitted by Blent under exclusive ownership.
    let replaced = "UsbFfs tcp:8890 tcp:2222\nUsbFfs tcp:9999 localabstract:external\n";
    std::fs::write(
        adb.with_extension("listing"),
        format!("{replaced}UsbFfs tcp:8891 tcp:18001\n"),
    )
    .unwrap();
    state.stop().await;
    assert!(ready);
    assert_eq!(listing(&adb), replaced, "T705: retired external mappings");
}

#[tokio::test]
async fn t705_partial_preparation_retains_unacknowledged_creation_until_observed() {
    if !private("monitor::route_tests::t705_partial_preparation_retains_unacknowledged_creation_until_observed") { return; }
    let foreign = "UsbFfs tcp:9999 localabstract:external\n";
    let (_root, mut state, adb) = fixture(foreign);
    std::fs::write(adb.with_extension("fail-input"), "").unwrap();
    state.prepare_primary();
    let owner = state.routes["USB"].clone();
    let (serial, result) = state.mutations.next().await;
    state.mutation_ready(serial, result);
    assert!(!state.ready.contains("USB"));
    let stopping = tokio::spawn(state.stop());
    assert!(owner.retire(adb.to_str().unwrap()).await.is_err());
    assert_eq!(listing(&adb), foreign);
    let log = std::fs::read_to_string(adb.with_extension("log")).unwrap();
    assert!(log.contains("--remove tcp:8890"));
    assert!(
        !log.contains("--remove tcp:8891"),
        "T705: attempted unowned removal"
    );
    // T1344: a failed native client cannot prove its delegated creator did nothing.
    // An empty listing must retain that outcome; publish it before fixture teardown.
    std::fs::remove_file(adb.with_extension("fail-input")).unwrap();
    std::fs::write(
        adb.with_extension("listing"),
        format!("{foreign}UsbFfs tcp:8891 tcp:18001\n"),
    )
    .unwrap();
    owner.retire(adb.to_str().unwrap()).await.unwrap();
    stopping.await.unwrap();
    assert_eq!(listing(&adb), foreign);
}

#[tokio::test]
async fn t705_repaired_routes_are_owned_and_retired() {
    if !private("monitor::route_tests::t705_repaired_routes_are_owned_and_retired") {
        return;
    }
    let external = "UsbFfs tcp:8890 tcp:18000\n";
    let (_root, mut state, adb) = fixture(external);
    prepare(&mut state).await;
    // Only the externally owned video mapping disappears. Blent's admitted
    // input endpoint remains intact until its original owner retires it.
    std::fs::write(adb.with_extension("listing"), "UsbFfs tcp:8891 tcp:18001\n").unwrap();
    state.check_forwarding();
    let (serial, result) = state.mutations.next().await;
    state.mutation_ready(serial, result);
    let repaired = listing(&adb).contains("tcp:8890 tcp:18000");
    state.stop().await;
    assert!(repaired);
    assert_eq!(listing(&adb), "");
}

#[tokio::test]
async fn t1346_offline_cleanup_retains_slot_and_revokes_the_old_lease() {
    if !private(
        "monitor::route_tests::t1346_offline_cleanup_retains_slot_and_revokes_the_old_lease",
    ) {
        return;
    }
    let (_root, mut state, adb) = fixture("");
    state.config.tablet = session::Spec {
        capture: Default::default(),
        ports: (0, 0),
        token: Some("a".repeat(64)),
        devices: (false, false, false),
    }
    .prepare(state.config.extra.mode_tx.clone())
    .unwrap()
    .tablet;
    prepare(&mut state).await;
    let owner = state.routes["USB"].clone();
    let lease = state.config.tablet.lease();
    assert!(lease.apply(|| ()));
    std::fs::write(adb.with_extension("offline"), "").unwrap();
    state.change_primary(&None);
    assert!(
        !lease.apply(|| ()),
        "T705: stale client survived retirement"
    );
    let early = tokio::time::timeout(Duration::from_millis(150), state.retiring.join_next()).await;
    let premature = early.is_ok();
    let retained = state.retiring_slots.contains(&0);
    std::fs::remove_file(adb.with_extension("offline")).unwrap();
    owner.retire(adb.to_str().unwrap()).await.unwrap();
    let retired = match early {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => state.retiring.join_next().await.unwrap().unwrap(),
    };
    state.retired(retired);
    assert!(listing(&adb).is_empty());
    state.stop().await;
    assert!(
        !premature,
        "T1346 returned a display ticket before route settlement"
    );
    assert!(retained, "T1346 missing reservation while offline");
}

#[tokio::test]
async fn t705_invalid_route_owners_fail_before_native_commands() {
    if !private("monitor::route_tests::t705_invalid_route_owners_fail_before_native_commands") {
        return;
    }
    let (_root, _state, adb) = fixture("");
    let command = adb.to_str().unwrap();
    for byte in 0..=31u8 {
        let serial = format!("bad{}serial", char::from(byte));
        assert!(RouteOwner::default()
            .prepare(&serial, (18000, 18001), command)
            .await
            .is_err());
    }
    for serial in [String::new(), "x".repeat(1025), "bad\u{7f}".into()] {
        assert!(RouteOwner::default()
            .prepare(&serial, (18000, 18001), command)
            .await
            .is_err());
    }
    for ports in [(0, 1), (1, 0), (1, 1), (65535, 65535)] {
        assert!(RouteOwner::default()
            .prepare("USB", ports, command)
            .await
            .is_err());
    }
    assert!(!adb.with_extension("log").exists());
}

#[tokio::test]
async fn t1346_failed_removals_keep_ticket_until_original_effects_arrive() {
    if !private(
        "monitor::route_tests::t1346_failed_removals_keep_ticket_until_original_effects_arrive",
    ) {
        return;
    }
    let foreign = "UsbFfs tcp:9999 localabstract:external\n";
    let (_root, mut state, adb) = fixture(foreign);
    prepare(&mut state).await;
    let owner = state.routes["USB"].clone();
    std::fs::write(adb.with_extension("fail-remove"), "").unwrap();
    state.change_primary(&None);
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let log = std::fs::read_to_string(adb.with_extension("log")).unwrap();
            if log.lines().filter(|line| line.contains("--remove")).count() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let early = tokio::time::timeout(Duration::from_millis(250), state.retiring.join_next()).await;
    let premature = early.is_ok();
    let retained = state.retiring_slots.contains(&0);
    // Reconnect supplies the original delegated removal outcomes, not permission
    // to issue duplicate unknown removers that could reach a replacement.
    std::fs::remove_file(adb.with_extension("fail-remove")).unwrap();
    std::fs::write(adb.with_extension("listing"), foreign).unwrap();
    owner.retire(adb.to_str().unwrap()).await.unwrap();
    let retired = match early {
        Ok(result) => result.unwrap().unwrap(),
        Err(_) => state.retiring.join_next().await.unwrap().unwrap(),
    };
    state.retired(retired);
    let log = std::fs::read_to_string(adb.with_extension("log")).unwrap();
    let removes = log.lines().filter(|line| line.contains("--remove")).count();
    let replacement = RouteOwner::default();
    replacement
        .prepare("USB", (18000, 18001), adb.to_str().unwrap())
        .await
        .unwrap();
    owner.retire(adb.to_str().unwrap()).await.unwrap();
    let replacement_preserved = listing(&adb).contains("tcp:8890 tcp:18000");
    replacement.retire(adb.to_str().unwrap()).await.unwrap();
    state.stop().await;
    assert!(
        !premature && retained,
        "T1346 failed removal released a display ticket"
    );
    assert_eq!(removes, 2, "T1346 each original remover is issued once");
    assert!(replacement_preserved);
    assert_eq!(listing(&adb), foreign);
}

async fn namespace_released() {
    use blent::usb::{Commands, NativeCommands};
    tokio::time::timeout(Duration::from_secs(3), async {
        while NativeCommands("unused".into())
            .display_route_lease("USB", (18000, 18001))
            .is_err()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("T1346 original native worker did not release its namespace");
}

#[tokio::test]
async fn t1346_monitor_loss_retains_cleanup_and_namespace_until_reconnect() {
    if !private(
        "monitor::route_tests::t1346_monitor_loss_retains_cleanup_and_namespace_until_reconnect",
    ) {
        return;
    }
    use blent::usb::{Commands, NativeCommands};
    let (_root, mut state, adb) = fixture("");
    prepare(&mut state).await;
    std::fs::write(adb.with_extension("offline"), "").unwrap();
    state.change_primary(&None);
    drop(state); // Includes cancellation of the retirement coroutine.
    let overlap = NativeCommands("unused".into()).display_route_lease("USB", (18000, 18001));
    let refused = overlap.is_err();
    drop(overlap);
    std::fs::remove_file(adb.with_extension("offline")).unwrap();
    namespace_released().await;
    assert!(refused, "T1346 caller loss permitted endpoint reuse");
    assert!(
        listing(&adb).is_empty(),
        "T1346 caller loss abandoned known mappings"
    );
}
