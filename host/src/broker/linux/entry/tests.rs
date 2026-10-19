use super::*;
use std::{future::Future, os::unix::fs::PermissionsExt};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .unwrap()
}
fn arguments() -> Vec<OsString> {
    [
        "blent-usb-broker",
        "--runtime",
        "/tmp/blent-entry-runtime-fixture",
        "--state",
        "/tmp/blent-entry-state-fixture",
        "--helper",
        "/tmp/blent-entry-helper-fixture",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn settings(root: &std::path::Path) -> Settings {
    let helper = root.join("helper");
    std::fs::write(
        &helper,
        b"#!/bin/sh\nprintf 'BLUI\\000\\000\\000\\001\\000\\000\\000\\000\\000\\000\\000\\000'\n",
    )
    .unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    Settings::new(root.join("runtime"), root.join("state"), helper)
}
#[tokio::test]
async fn t853_cli_defaults_and_overrides_reach_capacity_policy() {
    let defaults = Options::try_parse_from(arguments()).unwrap().settings();
    assert_eq!(defaults.control.clients, 16);
    assert_eq!(defaults.control.links, 4);
    assert_eq!(defaults.session.channel_bytes, 262144);
    let mut custom = arguments();
    custom.extend(
        [
            "--clients",
            "2",
            "--links",
            "1",
            "--devices",
            "3",
            "--registrations",
            "4",
            "--trust-peers",
            "5",
            "--block-bytes",
            "4096",
            "--channel-bytes",
            "4096",
            "--queued-messages",
            "8",
            "--quantum",
            "1024",
            "--io-timeout-ms",
            "200",
            "--setup-ms",
            "300",
            "--approval-ms",
            "400",
        ]
        .into_iter()
        .map(OsString::from),
    );
    let custom = Options::try_parse_from(custom).unwrap().settings();
    assert_eq!(
        (
            custom.control.clients,
            custom.control.links,
            custom.control.devices
        ),
        (2, 1, 3)
    );
    assert_eq!(
        (custom.registrations, custom.trust_peers, custom.block),
        (4, 5, 4096)
    );
    assert_eq!(
        (
            custom.session.channel_bytes,
            custom.session.queued_messages,
            custom.session.quantum
        ),
        (4096, 8, 1024)
    );
    assert_eq!(custom.session.io_timeout, Duration::from_millis(200));
    assert_eq!(
        (custom.control.setup_ms, custom.control.approval_ms),
        (300, 400)
    );
}
#[tokio::test]
async fn t853_cli_invalid_arguments_fail_without_starting_native_work() {
    assert!(bounded(run(vec![OsString::from("blent-usb-broker")]))
        .await
        .is_err());
    for value in ["", "-1", "x", "18446744073709551616"] {
        let mut args = arguments();
        args.extend([OsString::from("--links"), OsString::from(value)]);
        assert!(bounded(run(args)).await.is_err());
    }
    bounded(run(["blent-usb-broker", "--help"]
        .into_iter()
        .map(OsString::from)))
    .await
    .unwrap();
}
#[tokio::test]
async fn t853_entry_stop_and_failed_stop_retire_the_owner_socket() {
    let _guard = crate::broker::linux::server::authority::test_guard();
    let root = tempfile::tempdir().unwrap();
    let options = settings(root.path());
    let socket = options.runtime.join("broker.sock");
    bounded(serve(options.clone(), async { Ok(()) }))
        .await
        .unwrap();
    assert!(!socket.exists());
    let failure = bounded(serve(options, async {
        anyhow::bail!("fixture stop failure")
    }))
    .await
    .unwrap_err();
    assert_eq!(failure.to_string(), "fixture stop failure");
    assert!(!socket.exists());
}

#[test]
fn t1101_cli_exposes_cleanup_observation_budget() {
    for value in ["1", "5000", "60000"] {
        let mut args = arguments();
        args.extend([
            OsString::from("--retirement-timeout-ms"),
            OsString::from(value),
        ]);
        assert!(
            Options::try_parse_from(args).is_ok(),
            "T1101 cleanup budget unavailable: {value}"
        );
    }
}

fn cleanup_arguments(root: &std::path::Path, value: &str) -> Vec<OsString> {
    vec![
        "blent-usb-broker".into(),
        "--runtime".into(),
        root.join("runtime").into_os_string(),
        "--state".into(),
        root.join("state").into_os_string(),
        "--helper".into(),
        root.join("helper").into_os_string(),
        "--retirement-timeout-ms".into(),
        value.into(),
    ]
}
#[tokio::test]
async fn t1101_cli_cleanup_mapping_and_rejection_precede_native_effects() {
    let defaults = Options::try_parse_from(arguments()).unwrap().settings();
    assert_eq!(defaults.retirement_timeout, Duration::from_secs(5));
    let root = tempfile::tempdir().unwrap();
    for milliseconds in [1u64, 37, 5000, 60_000] {
        let options =
            Options::try_parse_from(cleanup_arguments(root.path(), &milliseconds.to_string()))
                .unwrap()
                .settings();
        assert_eq!(
            options.retirement_timeout,
            Duration::from_millis(milliseconds)
        );
    }
    for value in ["0", "60001", "18446744073709551615"] {
        let options = Options::try_parse_from(cleanup_arguments(root.path(), value))
            .unwrap()
            .settings();
        assert!(
            Server::open(options).await.is_err(),
            "T1101 invalid cleanup budget: {value}"
        );
        assert!(!root.path().join("runtime").exists());
        assert!(!root.path().join("state").exists());
    }
    for value in ["", "-1", "NaN", "1.5", "18446744073709551616"] {
        assert!(Options::try_parse_from(cleanup_arguments(root.path(), value)).is_err());
    }
}
