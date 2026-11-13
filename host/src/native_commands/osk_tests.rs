//! T1444: the OSK gate retains a remote Set until its original reply is observed.
use super::*;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
    Arc,
};

struct State {
    entered: AtomicBool,
    mode: AtomicI32,
    applied: Mutex<Vec<i32>>,
    release: tokio::sync::Notify,
}
struct Keyboard(Arc<State>);
#[zbus::interface(name = "org.kde.kwin.VirtualKeyboard")]
impl Keyboard {
    #[zbus(property, name = "available")]
    fn available(&self) -> bool {
        true
    }
    #[zbus(property, name = "mode")]
    fn mode(&self) -> i32 {
        self.0.mode.load(Ordering::Acquire)
    }
    #[zbus(property, name = "mode")]
    async fn set_mode(&mut self, mode: i32) {
        if !self.0.entered.swap(true, Ordering::AcqRel) {
            self.0.release.notified().await;
        }
        self.0.mode.store(mode, Ordering::Release);
        let mut applied = self.0.applied.lock().unwrap();
        assert!(
            applied.len() < 2,
            "fixture permits exactly disable and restore"
        );
        applied.push(mode);
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
            mode: AtomicI32::new(1),
            applied: Mutex::new(Vec::new()),
            release: tokio::sync::Notify::new(),
        });
        let bus = zbus::connection::Builder::session()
            .unwrap()
            .name("org.kde.KWin")
            .unwrap()
            .serve_at("/VirtualKeyboard", Keyboard(state.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        Self {
            state,
            bus: Some(bus),
        }
    }
    async fn close(mut self) {
        self.bus.take().unwrap().close().await.unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.release.notify_one();
    }
}
fn private() -> bool {
    if std::env::var_os("BLENT_T1444_PRIVATE_OSK").is_some() {
        return true;
    }
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new("dbus-run-session")
        .arg("--")
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native_commands::osk_tests::t1444_cancelled_osk_set_retains_gate_until_original_ack",
            "--nocapture",
        ])
        .env("BLENT_T1444_PRIVATE_OSK", "1")
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path())
        .env("XDG_RUNTIME_DIR", root.path())
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
async fn until(done: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn synchronize(
    devices: Arc<AtomicUsize>,
    proofs: Arc<Mutex<Vec<Completion>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        crate::osk::sync_touch_state(&devices, &|proof| {
            proofs.lock().unwrap().push(proof);
            Ok(())
        })
        .await;
    })
}
#[tokio::test]
async fn t1444_cancelled_osk_set_retains_gate_until_original_ack() {
    if !private() {
        return;
    }
    let fixture = Fixture::new().await;
    let devices = Arc::new(AtomicUsize::new(1));
    let proofs = Arc::new(Mutex::new(Vec::new()));
    let disable = synchronize(devices.clone(), proofs.clone());
    until(|| fixture.state.entered.load(Ordering::Acquire)).await;
    disable.abort();
    assert!(disable.await.unwrap_err().is_cancelled());
    devices.store(0, Ordering::Release);
    let restore = synchronize(devices, proofs.clone());
    tokio::time::sleep(Duration::from_millis(150)).await;
    let premature = proofs.lock().unwrap()[0].is_complete();
    let early_restore = restore.is_finished();
    assert!(fixture.state.applied.lock().unwrap().is_empty());
    fixture.state.release.notify_one();
    restore.await.unwrap();
    until(|| proofs.lock().unwrap().iter().all(Completion::is_complete)).await;
    assert_eq!(*fixture.state.applied.lock().unwrap(), [0, 1]);
    assert_eq!(proofs.lock().unwrap().len(), 2);
    assert!(!std::path::Path::new(&std::env::var("HOME").unwrap())
        .join(".local/share/blent/osk-restore")
        .exists());
    fixture.close().await;
    assert!(
        !premature,
        "T1444 shared gate retired before original remote Set reply"
    );
    assert!(!early_restore, "T1444 restoration overtook original Set");
}
