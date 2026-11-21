use super::*;
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Instant,
};

pub(super) async fn pause() {
    let Some(root) = std::env::var_os("BLENT_T1637_STARTUP") else {
        return;
    };
    let root = Path::new(&root);
    std::fs::write(root.join("ready"), b"startup admitted").unwrap();
    while !root.join("release").exists() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn t1637_startup_child() {
    let Some(root) = std::env::var_os("BLENT_T1637_STARTUP") else {
        return;
    };
    let root = PathBuf::from(root);
    let args = vec![
        "blent-usb-broker".into(),
        "--runtime".into(),
        root.join("runtime").into_os_string(),
        "--state".into(),
        root.join("state").into_os_string(),
        "--helper".into(),
        root.join("absent-helper").into_os_string(),
    ];
    run(args).await.unwrap();
    assert!(
        root.join("state/identity.bin").is_file(),
        "T1637 original startup did not finish"
    );
    assert!(
        !root.join("runtime/broker.sock").exists(),
        "T1637 original listener remained owned"
    );
}

struct Fixture {
    child: Child,
    root: tempfile::TempDir,
}
impl Fixture {
    fn start() -> Self {
        let root = tempfile::tempdir().unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "broker::linux::entry::startup_tests::t1637_startup_child",
                "--nocapture",
            ])
            .env("BLENT_T1637_STARTUP", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Self { child, root }
    }
    fn ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.root.path().join("ready").exists() {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "T1637 startup child exited early"
            );
            assert!(
                Instant::now() < deadline,
                "T1637 startup handshake unavailable"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn signal_and_finish(&mut self, signal: i32) {
        assert_eq!(unsafe { libc::kill(self.child.id() as i32, signal) }, 0);
        std::fs::write(self.root.path().join("release"), b"finish original startup").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "T1637 signal {signal} killed startup: {status}"
                );
                return;
            }
            assert!(
                Instant::now() < deadline,
                "T1637 signal was lost during startup"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn t1637_signals_during_startup_wait_for_original_owner_then_shutdown() {
    for signal in [libc::SIGTERM, libc::SIGINT] {
        let mut fixture = Fixture::start();
        fixture.ready();
        fixture.signal_and_finish(signal);
    }
}
