use super::*;
use crate::native_commands::{CommandPort, ObservedCommands};
use std::{os::fd::AsRawFd, sync::Arc, time::Duration};

fn private(name: &str) -> bool {
    if std::env::var_os("BLENT_T1641_NATIVE_X11").is_some() {
        return true;
    }
    let output = std::process::Command::new("xvfb-run")
        .args(["-a", "-s", "-screen 0 128x96x24 -nolisten tcp"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("BLENT_T1641_NATIVE_X11", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}
fn request() -> Mapping {
    Mapping::new("Virtual core pointer", "screen").unwrap()
}

#[tokio::test]
async fn t1641_original_x11_error_settles_but_transport_loss_does_not() {
    if !private(
        "native_commands::x11::tests::t1641_original_x11_error_settles_but_transport_loss_does_not",
    ) {
        return;
    }
    let (connection, screen) = RustConnection::connect(None).unwrap();
    let mut invalid = prepare(&connection, screen, request()).unwrap();
    invalid.device = u16::MAX;
    let rejected = apply(&connection, invalid);
    assert!(rejected.settled && rejected.result.is_err());
    let valid = prepare(&connection, screen, request()).unwrap();
    let succeeded = apply(&connection, valid);
    assert!(succeeded.settled && succeeded.result.is_ok());
    let pending = prepare(&connection, screen, request()).unwrap();
    assert_eq!(
        unsafe { libc::shutdown(connection.stream().as_raw_fd(), libc::SHUT_RDWR) },
        0
    );
    let lost = apply(&connection, pending);
    assert!(!lost.settled);
    assert!(lost.result.is_err());
    let panicked = joined(|| panic!("T1641 original worker fault control")).await;
    assert!(!panicked.settled);
}

#[tokio::test]
async fn t1641_predispatch_errors_retire_without_a_mutation() {
    if !private("native_commands::x11::tests::t1641_predispatch_errors_retire_without_a_mutation") {
        return;
    }
    let (connection, screen) = RustConnection::connect(None).unwrap();
    assert!(atom(&connection, b"BLENT_T1641_missing_atom").is_err());
    for name in ["missing-device-t1641", "Virtual core keyboard"] {
        let report = execute(Mapping::new(name, "screen").unwrap()).await;
        assert!(report.settled && report.result.is_err());
    }
    for _ in 0..2 {
        assert!(std::process::Command::new("/usr/bin/xinput")
            .args(["create-master", "blent-t1641"])
            .status()
            .unwrap()
            .success());
    }
    assert!(device(&connection, "blent-t1641 pointer").is_err());
    assert!(prepare(
        &connection,
        screen,
        Mapping::new("blent-t1641 pointer", "screen").unwrap()
    )
    .is_err());
    std::env::set_var("DISPLAY", "invalid-t1641-display");
    let report = execute(request()).await;
    assert!(report.settled && report.result.is_err());
}

async fn until(done: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1641_cancelled_waiter_retains_original_x11_worker_until_fence() {
    if !private("native_commands::x11::tests::t1641_cancelled_waiter_retains_original_x11_worker_until_fence") { return; }
    let (gate, _) = RustConnection::connect(None).unwrap();
    gate.grab_server().unwrap().check().unwrap();
    let commands = Arc::new(ObservedCommands::default());
    let work = commands.clone();
    let caller =
        tokio::spawn(async move { work.map_output("Virtual core pointer", "screen").await });
    until(|| commands.state().mutation.is_some()).await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !commands.complete(),
        "T1641 caller loss retired a blocked original X11 worker"
    );
    gate.ungrab_server().unwrap().check().unwrap();
    until(|| commands.complete()).await;
    assert!(commands
        .map_output("Virtual core pointer", "screen")
        .await
        .is_ok());
    until(|| commands.complete()).await;
}

#[test]
fn t1641_mapping_names_and_property_shapes_are_bounded() {
    for name in ["".into(), "a\0b".into(), "x".repeat(4097)] {
        assert!(Mapping::new(&name, "screen").is_err());
        assert!(Mapping::new("device", &name).is_err());
    }
    assert!(Mapping::new(&"x".repeat(4096), "screen").is_ok());
    for length in 0..=16 {
        let reply = xinput::XIGetPropertyReply {
            sequence: 1,
            length: 0,
            type_: 7,
            bytes_after: 0,
            num_items: length,
            items: xinput::XIGetPropertyItems::Data32(vec![0; length as usize]),
        };
        assert_eq!(valid_property(&reply, 7), length == 9);
        assert!(!valid_property(&reply, 8));
    }
    let mut reply = xinput::XIGetPropertyReply {
        sequence: 1,
        length: 0,
        type_: 7,
        bytes_after: 1,
        num_items: 9,
        items: xinput::XIGetPropertyItems::Data8(vec![0; 9]),
    };
    assert!(!valid_property(&reply, 7));
    reply.bytes_after = 0;
    assert!(!valid_property(&reply, 7));
}
