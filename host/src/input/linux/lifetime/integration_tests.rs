//! T1508: original input ownership spans rejection, retry, cancellation and drain.
use super::*;
use crate::native_commands::{CommandPort, ObservedCommands, Property};
use std::time::Duration;
use x11rb::{protocol::xproto::ConnectionExt, rust_connection::RustConnection};

fn private() -> bool {
    if std::env::var_os("BLENT_T1508_DRAIN").is_some() {
        return true;
    }
    let output = std::process::Command::new("xvfb-run")
        .args(["-a", "-s", "-screen 0 128x96x24 -nolisten tcp"])
        .arg("dbus-run-session")
        .arg("--")
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "input::linux::lifetime::integration_tests::t1508_rejected_mutations_recover_and_original_input_drains",
            "--nocapture",
        ])
        .env("BLENT_T1508_DRAIN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    false
}

struct Fixture(i32);
#[zbus::interface(name = "org.blent.Fixture")]
impl Fixture {
    #[zbus(property)]
    fn mode(&self) -> i32 {
        self.0
    }
    #[zbus(property)]
    fn set_mode(&mut self, value: i32) {
        self.0 = value;
    }
}
async fn service() -> zbus::Connection {
    zbus::connection::Builder::session()
        .unwrap()
        .name("org.kde.KWin")
        .unwrap()
        .serve_at("/fixture", Fixture(0))
        .unwrap()
        .build()
        .await
        .unwrap()
}
async fn until(done: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn recovered(commands: &ObservedCommands) {
    assert!(commands
        .map_output("Virtual core pointer", "absent-t1508")
        .await
        .is_err());
    until(|| commands.complete()).await;
    assert!(commands
        .map_output("Virtual core pointer", "screen")
        .await
        .is_ok());
    until(|| commands.complete()).await;
    for (name, succeeds) in [("Missing", false), ("Mode", true)] {
        let result = commands
            .set_property(Property::new("/fixture", "org.blent.Fixture", name, "i", "1").unwrap())
            .await;
        assert_eq!(result.is_ok(), succeeds);
        until(|| commands.complete()).await;
    }
}
async fn cancelled(commands: Arc<ObservedCommands>) {
    let work = commands.clone();
    let caller =
        tokio::spawn(async move { work.map_output("Virtual core pointer", "screen").await });
    until(|| !commands.complete()).await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!commands.complete());
}

#[tokio::test]
async fn t1508_rejected_mutations_recover_and_original_input_drains() {
    if !private() {
        return;
    }
    let bus = service().await;
    let lifetime = Lifetime::new().unwrap();
    lifetime.claim(1508).unwrap();
    let live = lifetime.admit_payload().unwrap();
    let commands = lifetime.commands();
    recovered(&commands).await;
    let (gate, _) = RustConnection::connect(None).unwrap();
    gate.grab_server().unwrap().check().unwrap();
    cancelled(commands).await;
    let payload = live.retire(super::super::InjectDevices::empty(), false);
    let original = lifetime.completion();
    drop(lifetime);
    let drain = crate::process_retirement::PROCESS.drain();
    tokio::pin!(drain);
    assert!(futures_util::poll!(drain.as_mut()).is_pending());
    assert!(!payload.is_complete());
    assert!(!original.is_complete());
    assert!(Identity::claim(1508).is_err());
    gate.ungrab_server().unwrap().check().unwrap();
    tokio::time::timeout(Duration::from_secs(3), drain)
        .await
        .unwrap();
    assert!(payload.is_complete());
    assert!(original.is_complete());
    drop(Identity::claim(1508).unwrap());
    bus.close().await.unwrap();
}
