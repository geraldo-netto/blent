use super::*;
use blent_config::{
    display_control::{Command as DisplayCommand, Reply, Snapshot, Target},
    linux::display_control::Server,
    tablets::Identity,
    workspace::{Device, Presence},
};

#[test]
fn t1273_inventory_keeps_exact_identity_and_rejects_invalid_or_exhausted_inputs() {
    let text = "List of devices attached\nfirst device model:Same_Name\nsecond device model:Same_Name\noffline offline\nunauthorized unauthorized\n\n";
    let devices = inventory(text).unwrap();
    assert_eq!(devices.len(), 2);
    assert_ne!(devices[0].identity(), devices[1].identity());
    assert_eq!(devices[0].name(), devices[1].name());
    assert_eq!(devices[0].presence(), Presence::Unknown);
    assert_eq!(inventory("first device\nfirst device\n").unwrap().len(), 1);
    assert!(inventory(&format!("{} device", "a".repeat(1025))).is_err());
    assert!(inventory("bad\0serial device").is_err());
    let oversized = (0..1025)
        .map(|index| format!("tablet-{index} device\n"))
        .collect::<String>();
    assert!(inventory(&oversized).is_err());
    let mut status = Status {
        tablets: devices.clone(),
        ..Default::default()
    };
    merge_inventory(&mut status, &devices);
    assert_eq!(status.tablets.len(), 2);
    let capacity: Vec<_> = (0..1024)
        .map(|index| {
            Device::new(
                Identity::Adb(format!("tablet-{index}")),
                "Tablet".into(),
                Presence::Unknown,
                false,
            )
            .unwrap()
        })
        .collect();
    merge_inventory(&mut status, &capacity);
    assert_eq!(status.tablets.len(), 1024);
    assert!(status.inventory_error.is_some());
}

#[test]
fn t1273_display_adapter_and_actions_use_exact_live_owner_and_target() {
    let (ready, received) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut server = Server::bind(8).unwrap();
            ready.send(()).unwrap();
            for index in 0..5 {
                let call = server.next().await.unwrap();
                if index == 0 { assert!(matches!(call.command, DisplayCommand::Inspect {})); }
                else { assert!(matches!(&call.command, DisplayCommand::Set { target: Target::Adb(serial), .. } if serial == "first")); }
                server.respond(call, Reply { snapshot: Snapshot { owner: "a".repeat(64), assignments: vec![] }, error: (index == 2).then(|| "fixture admission rejected".into()) });
            }
            assert!(tokio::time::timeout(std::time::Duration::from_millis(100), server.next()).await.is_err());
            server.close().await;
        });
    });
    received.recv().unwrap();
    let mut source = Platform::default();
    let mut status = Status {
        daemon_pid: std::process::id(),
        ..Default::default()
    };
    source.query_display(&mut status);
    assert!(status.display.is_some());
    assert!(status.display_error.is_none());
    let command = DisplayCommand::Set {
        owner: "a".repeat(64),
        target: Target::Adb("first".into()),
        enabled: true,
        expected: None,
    };
    assert_eq!(
        crate::platform::display_command(std::process::id(), &command),
        "Display request accepted; waiting for tablet state."
    );
    assert_eq!(
        crate::platform::display_command(std::process::id(), &command),
        "fixture admission rejected"
    );
    status.daemon_pid = u32::MAX;
    source.query_display(&mut status);
    assert!(status.display.is_none());
    assert!(status.display_error.is_some());
    assert!(crate::platform::display_command(u32::MAX, &command).contains("unconfirmed"));
    exercise_display_actions();
    worker.join().unwrap();
}

fn exercise_display_actions() {
    use blent_config::display_control::{Assignment, Lease, Phase};
    let (_root, mut app) = crate::tests::settings_test_app(crate::Tab::Overview);
    let mut status = Status {
        daemon_pid: std::process::id(),
        display: Some(Snapshot {
            owner: "a".repeat(64),
            assignments: vec![Assignment {
                target: Target::Adb("first".into()),
                lease: Lease {
                    slot: 0,
                    generation: 1,
                },
                phase: Phase::Assigned,
            }],
        }),
        ..Default::default()
    };
    let target = Identity::Adb("first".into());
    for enabled in [false, true] {
        app.display_action(&status, &target, enabled);
        while app.busy() {
            app.poll_action();
            std::thread::yield_now();
        }
        assert_eq!(
            app.message,
            "Display request accepted; waiting for tablet state."
        );
        status.display.as_mut().unwrap().assignments.clear();
    }
    status.display = None;
    app.display_action(&status, &target, true);
    assert!(!app.busy());
}
