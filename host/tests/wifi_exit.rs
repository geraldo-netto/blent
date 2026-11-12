//! T1365: graceful process-entry teardown retains original Wi-Fi cleanup.
use blent::usb::{Adb, CommandFuture, Commands};
use blent_config::{commands::SyncCommandExt, storage::ConfigStore};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
struct State {
    present: AtomicBool,
    removing: tokio::sync::Notify,
    store: ConfigStore,
}
#[derive(Clone)]
struct Fake(Arc<State>);
#[cfg(unix)]
fn success(text: &str) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;
    std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: text.as_bytes().to_vec(),
        stderr: vec![],
    }
}
#[cfg(windows)]
fn success(text: &str) -> std::process::Output {
    use std::os::windows::process::ExitStatusExt;
    std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: text.as_bytes().to_vec(),
        stderr: vec![],
    }
}
impl Commands for Fake {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            match args[0].as_str() {
                "devices" => Ok(success(if self.0.present.load(Ordering::Acquire) {
                    "List of devices attached\nhost:5\tdevice\n"
                } else {
                    "List of devices attached\n"
                })),
                "connect" => {
                    self.0.present.store(true, Ordering::Release);
                    self.0.store.update(|config| {
                        config.wifi_address.clear();
                        Ok(())
                    })?;
                    Ok(success("connected to host:5"))
                }
                "disconnect" => {
                    self.0.removing.notify_one();
                    anyhow::bail!("T1365 original removal remains pending")
                }
                _ => panic!("unexpected command"),
            }
        })
    }
}
#[test]
fn t1365_exit_fixture() {
    if std::env::var_os("BLENT_T1365_EXIT").is_none() {
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let store = ConfigStore::new(root.path().join("config.toml"));
    store
        .update(|config| {
            config.wifi_address = "host:5".into();
            Ok(())
        })
        .unwrap();
    let state = Arc::new(State {
        present: AtomicBool::new(false),
        removing: Default::default(),
        store,
    });
    let worker = state.clone();
    let (done, result) = std::sync::mpsc::channel();
    let (leaving, leaving_rx) = std::sync::mpsc::channel();
    let native = std::thread::spawn(move || {
        let value = blent::runtime_owner::run(async {
            let adb = Adb(Fake(worker.clone()));
            tokio::select! {
                _ = blent::wifi::reconnect(&adb, &worker.store) => panic!("cleanup finished before its effect"),
                _ = worker.removing.notified() => {}
            }
            leaving.send(()).unwrap();
            73
        }).unwrap();
        done.send(value).unwrap();
    });
    leaving_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let premature = result.recv_timeout(Duration::from_millis(200));
    state.present.store(false, Ordering::Release);
    let returned = premature
        .as_ref()
        .copied()
        .unwrap_or_else(|_| result.recv_timeout(Duration::from_secs(3)).unwrap());
    native.join().unwrap();
    assert!(
        premature.is_err(),
        "T1365 process entry returned with original Wi-Fi removal pending"
    );
    assert_eq!(returned, 73);
}
#[test]
fn t1365_process_exit_waits_for_original_cleanup() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "t1365_exit_fixture", "--nocapture"])
        .env("BLENT_T1365_EXIT", "1")
        .env("BLENT_RUNTIME_STOP_MS", "20")
        .output_timeout(Duration::from_secs(10))
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
