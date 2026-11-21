//! T1444: caller cancellation/deadline does not settle a delegated mutation.
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
struct State {
    entered: AtomicBool,
    applied: AtomicBool,
    release: tokio::sync::Notify,
}
struct Device(Arc<State>);
#[zbus::interface(name = "org.kde.KWin.InputDevice")]
impl Device {
    #[zbus(property, name = "outputName")]
    fn output_name(&self) -> &str {
        "before"
    }
    #[zbus(property, name = "outputName")]
    async fn set_output_name(&mut self, _value: &str) {
        self.0.entered.store(true, Ordering::Release);
        self.0.release.notified().await;
        self.0.applied.store(true, Ordering::Release);
    }
}
struct Keyboard;
#[zbus::interface(name = "org.kde.kwin.VirtualKeyboard")]
impl Keyboard {
    #[zbus(property, name = "available")]
    fn available(&self) -> bool {
        true
    }
}
struct Fixture {
    state: Arc<State>,
    bus: Option<zbus::Connection>,
}
impl Fixture {
    async fn new() -> Self {
        let state = Arc::new(State {
            entered: AtomicBool::new(false),
            applied: AtomicBool::new(false),
            release: tokio::sync::Notify::new(),
        });
        let bus = zbus::connection::Builder::session()
            .unwrap()
            .name("org.kde.KWin")
            .unwrap()
            .serve_at("/VirtualKeyboard", Keyboard)
            .unwrap()
            .serve_at("/org/kde/KWin/InputDevice/event41", Device(state.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        Self {
            state,
            bus: Some(bus),
        }
    }
    async fn finish(mut self, commands: &ObservedCommands) {
        self.state.release.notify_one();
        until(|| self.state.applied.load(Ordering::Acquire)).await;
        until(|| commands.complete()).await;
        self.bus.take().unwrap().close().await.unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.release.notify_one();
    }
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
async fn request(commands: &ObservedCommands) -> io::Result<()> {
    commands
        .set_property(
            Property::new(
                "/org/kde/KWin/InputDevice/event41",
                "org.kde.KWin.InputDevice",
                "outputName",
                "s",
                "after",
            )
            .unwrap(),
        )
        .await
}
#[tokio::test]
async fn t1444_cancelled_mutation_retains_original_reply_and_debt() {
    if !crate::activity::private_bus_test(
        "native_commands::mutation_tests::t1444_cancelled_mutation_retains_original_reply_and_debt",
    ) {
        return;
    }
    let fixture = Fixture::new().await;
    let commands = Arc::new(ObservedCommands::default());
    let work = commands.clone();
    let caller = tokio::spawn(async move { request(&work).await });
    until(|| fixture.state.entered.load(Ordering::Acquire)).await;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let premature = commands.complete();
    assert!(!fixture.state.applied.load(Ordering::Acquire));
    fixture.finish(&commands).await;
    assert!(
        !premature,
        "T1444 caller cancellation retired an unacknowledged remote Set"
    );
}
#[tokio::test]
async fn t1444_deadline_ends_caller_wait_but_retains_original_mutation() {
    if !crate::activity::private_bus_test("native_commands::mutation_tests::t1444_deadline_ends_caller_wait_but_retains_original_mutation") { return; }
    let fixture = Fixture::new().await;
    let commands = ObservedCommands::default();
    assert_eq!(
        request(&commands).await.unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert!(fixture.state.entered.load(Ordering::Acquire));
    tokio::time::sleep(Duration::from_millis(150)).await;
    let premature = commands.complete();
    assert!(!fixture.state.applied.load(Ordering::Acquire));
    fixture.finish(&commands).await;
    assert!(
        !premature,
        "T1444 caller deadline retired an unacknowledged remote Set"
    );
}

#[tokio::test]
async fn t1444_query_only_port_cannot_start_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("not-created");
    let error = SystemCommands
        .mutate(
            "/bin/sh",
            &["-c", "touch \"$1\"", "fixture", marker.to_str().unwrap()],
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(!marker.exists());
    let error = SystemCommands
        .set_property(Property::new("/fixture", "org.blent.Fixture", "mode", "i", "1").unwrap())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
}
#[tokio::test]
async fn t1444_failed_spawn_is_no_effect_and_original_success_releases() {
    let commands = ObservedCommands::default();
    for program in ["", "\0", "/nonexistent-blent-t1444"] {
        assert!(commands.mutate(program, &[]).await.is_err());
        until(|| commands.complete()).await;
    }
    assert!(commands
        .mutate("/bin/true", &[])
        .await
        .unwrap()
        .status
        .success());
    until(|| commands.complete()).await;
}
#[tokio::test]
async fn t1444_native_exit_without_ack_retains_bounded_quarantine() {
    for args in [&["-c", "exit 7"][..], &["-c", "kill -TERM $$"][..]] {
        let commands = ObservedCommands::default();
        let result = commands.mutate("/bin/sh", args).await.unwrap();
        assert!(!result.status.success());
        until(|| commands.state().originals.complete()).await;
        assert!(!commands.complete());
        let successor = commands.mutate("/bin/true", &[]);
        tokio::pin!(successor);
        assert!(futures_util::poll!(&mut successor).is_pending());
        assert!(commands.reserve().is_err());
        assert!(
            !commands.complete(),
            "T1444 native exit did not acknowledge remote outcome"
        );
    }
}

#[tokio::test]
async fn t1640_rejected_property_set_retires_and_allows_successor() {
    if !crate::activity::private_bus_test(
        "native_commands::mutation_tests::t1640_rejected_property_set_retires_and_allows_successor",
    ) {
        return;
    }
    let fixture = Fixture::new().await;
    let commands = ObservedCommands::default();
    assert!(
        !crate::kwin::set_property_with(
            &commands,
            "/org/kde/KWin/InputDevice/event41",
            "org.kde.KWin.InputDevice",
            "missingProperty",
            "s",
            "after"
        )
        .await
    );
    tokio::time::timeout(Duration::from_millis(200), async {
        while !commands.complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("T1640 original negative reply never settled");
    fixture.state.release.notify_one();
    assert!(
        crate::kwin::set_property_with(
            &commands,
            "/org/kde/KWin/InputDevice/event41",
            "org.kde.KWin.InputDevice",
            "outputName",
            "s",
            "after"
        )
        .await
    );
    until(|| commands.complete()).await;
    assert!(fixture.state.applied.load(Ordering::Acquire));
}
