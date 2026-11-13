//! Private D-Bus: exact command completion is not remote mutation completion.
use blent_config::{commands::AsyncCommandExt, retirement::Completion};
use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
struct State {
    entered: AtomicBool,
    applied: AtomicBool,
    release: tokio::sync::Notify,
}
struct Device(Arc<State>);
#[zbus::interface(name = "org.kde.KWin.InputDevice")]
impl Device {
    #[zbus(property, name = "outputName")]
    fn output_name(&self) -> &str { "before" }
    #[zbus(property, name = "outputName")]
    async fn set_output_name(&mut self, _name: &str) {
        self.0.entered.store(true, Ordering::Release);
        self.0.release.notified().await;
        self.0.applied.store(true, Ordering::Release);
    }
}
async fn settle(proofs: &Mutex<Vec<Completion>>) {
    while !proofs.lock().unwrap().iter().all(Completion::is_complete) {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn execute(state: &State) {
    let proofs = Mutex::new(Vec::new());
    let result = tokio::process::Command::new("/usr/bin/busctl")
        .args(["--user", "set-property", "org.kde.KWin", "/org/kde/KWin/InputDevice/event41",
            "org.kde.KWin.InputDevice", "outputName", "s", "after"])
        .output_input_timeout_observed(None, Duration::from_secs(1), &|proof| {
            proofs.lock().unwrap().push(proof);
        }).await;
    assert!(state.entered.load(Ordering::Acquire));
    settle(&proofs).await;
    assert_eq!(proofs.lock().unwrap().len(), 2);
    assert!(!state.applied.load(Ordering::Acquire));
    println!("command_result={result:?}; original_child_and_collector_complete=true; remote_applied=false");
    state.release.notify_one();
    while !state.applied.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    println!("remote_applied_after_both_native_receipts=true");
}
async fn run() {
    let state = Arc::new(State { entered: AtomicBool::new(false), applied: AtomicBool::new(false), release: tokio::sync::Notify::new() });
    let server = zbus::connection::Builder::session().unwrap().name("org.kde.KWin").unwrap()
        .serve_at("/org/kde/KWin/InputDevice/event41", Device(state.clone())).unwrap()
        .build().await.unwrap();
    execute(&state).await;
    server.close().await.unwrap();
    println!("private_server_closed=true");
}
fn main() {
    assert!(std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some());
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(run());
}
