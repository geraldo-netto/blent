use super::*;
use crate::usb::{CommandFuture, RouteCommand, RouteFuture};
use std::collections::VecDeque;
struct Script(Mutex<VecDeque<RouteCommand>>);
impl Commands for Script {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("route commands require fenced outcomes")
    }
    fn execute_route(&self, _: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async { self.0.lock().unwrap().pop_front().unwrap() })
    }
}
fn reply(code: u32, text: &str) -> RouteCommand {
    RouteCommand {
        output: Ok(crate::wifi::tests::output(code, text)),
        started: true,
    }
}
fn listing(rows: &str) -> RouteCommand {
    reply(0, &format!("List of devices attached\n{rows}"))
}
fn scripted(commands: Vec<RouteCommand>) -> Adb<Script> {
    Adb(Script(Mutex::new(commands.into())))
}
#[tokio::test]
async fn t1363_removal_waits_for_effect_and_retries_only_proven_nonstarts() {
    let adb = scripted(vec![
        listing("HOST:0005\tconnecting\n"),
        RouteCommand {
            output: Err(anyhow::anyhow!("spawn rejected")),
            started: false,
        },
        listing("host:5\tunauthorized\n"),
        reply(0, "disconnected"),
        listing(""),
    ]);
    let mut state = Creation::Acknowledged;
    assert!(once(&adb, "host:5", &mut state).await.is_err());
    assert_eq!(state, Creation::Acknowledged);
    assert!(!once(&adb, "host:5", &mut state).await.unwrap());
    assert!(once(&adb, "host:5", &mut state).await.unwrap());
    assert!(adb.0 .0.lock().unwrap().is_empty());
}
#[tokio::test]
async fn t1363_unknown_nonzero_removal_is_not_reissued() {
    let adb = scripted(vec![
        listing("host:5\toffline\n"),
        reply(1, ""),
        listing("host:5\toffline\n"),
        listing(""),
    ]);
    let mut state = Creation::Acknowledged;
    assert!(once(&adb, "host:5", &mut state).await.is_err());
    assert_eq!(state, Creation::RemovalPending);
    assert!(!once(&adb, "host:5", &mut state).await.unwrap());
    assert!(once(&adb, "host:5", &mut state).await.unwrap());
}
#[tokio::test]
async fn t1363_invalid_snapshots_never_discharge_cleanup() {
    for text in [
        "",
        "error",
        "List of devices attached\nhost:5\tdevice\nhost:5\toffline",
        "List of devices attached\nhost:5\tbogus",
    ] {
        let adb = scripted(vec![reply(0, text)]);
        assert!(once(&adb, "host:5", &mut Creation::Acknowledged)
            .await
            .is_err());
    }
    let mut invalid = reply(0, "");
    invalid.output.as_mut().unwrap().stdout = vec![255];
    for command in [invalid, reply(1, ""), reply(0, &"x".repeat(65537))] {
        assert!(observe(&scripted(vec![command]), "host:5").await.is_err());
    }
}
#[test]
fn t1363_endpoint_keys_cover_valid_and_invalid_forms() {
    use crate::wifi::endpoint_key;
    for (input, expected) in [
        ("HOST:0005", "host:5"),
        ("[2001:0db8::1]:5", "[2001:db8::1]:5"),
        ("host:invalid", "host:invalid"),
        ("", ""),
    ] {
        assert_eq!(endpoint_key(input), expected);
    }
}
