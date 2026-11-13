use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct State {
    connected: AtomicBool,
    started: AtomicUsize,
    retired: AtomicUsize,
    identities: AtomicUsize,
    settled: AtomicUsize,
    release: tokio::sync::Notify,
}
#[derive(Clone, Default)]
struct Stalled(Arc<State>);
struct NativeLease(Arc<State>);
impl Drop for NativeLease {
    fn drop(&mut self) {
        self.0.settled.fetch_add(1, Ordering::SeqCst);
    }
}
impl Commands for Stalled {
    fn route_lease(&self, _: &str) -> Result<Box<dyn Send>> {
        Ok(Box::new(NativeLease(self.0.clone())))
    }

    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            let text = match args[0].as_str() {
                "devices" => format!(
                    "List of devices attached\nUSB\tdevice\n{}",
                    if self.0.connected.load(Ordering::SeqCst) {
                        "192.0.2.138:5555\tdevice\n"
                    } else {
                        ""
                    }
                ),
                "connect" => {
                    self.0.started.fetch_add(1, Ordering::SeqCst);
                    self.0.release.notified().await;
                    self.0.connected.store(true, Ordering::SeqCst);
                    "connected to 192.0.2.138:5555".into()
                }
                "disconnect" => {
                    self.0.connected.store(false, Ordering::SeqCst);
                    self.0.retired.fetch_add(1, Ordering::SeqCst);
                    String::new()
                }
                _ => {
                    self.0.identities.fetch_add(1, Ordering::SeqCst);
                    "healthy-tablet".into()
                }
            };
            Ok(output(0, &text))
        })
    }
}
fn fixture() -> (Adb<Stalled>, network::Network, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    store
        .update(|c| {
            c.wifi_address = "192.0.2.138:5555".into();
            Ok(())
        })
        .unwrap();
    (Adb(Stalled::default()), network::Network::new(store), root)
}
async fn observed(counter: &AtomicUsize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while counter.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("T1387: fixture event missing");
}

#[tokio::test]
async fn t1387_stalled_wifi_preserves_usb_progress_and_cancellation_cleanup() {
    let (adb, mut network, _root) = fixture();
    let first = tokio::time::timeout(Duration::from_secs(1), network.devices(&adb, &[])).await;
    observed(&adb.0 .0.started).await;
    let queries = adb.0 .0.identities.load(Ordering::SeqCst);
    for _ in 0..5 {
        network.devices(&adb, &[]).await;
    }
    assert_eq!(adb.0 .0.started.load(Ordering::SeqCst), 1);
    network.stop().await;
    // The original creator can finish after its caller and scheduler stop.
    assert_eq!(adb.0 .0.retired.load(Ordering::SeqCst), 0);
    adb.0 .0.release.notify_one();
    observed(&adb.0 .0.retired).await;
    observed(&adb.0 .0.settled).await;
    assert!(!adb.0 .0.connected.load(Ordering::SeqCst));
    assert_eq!(
        first.expect("T1387: optional Wi-Fi blocked USB"),
        Some(vec!["USB".into()])
    );
    assert!(queries > 0, "T1387: USB identity never scheduled");
}

pub(super) async fn wait_for_attachment(monitor: &mut monitor::Monitor<Fake>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while monitor.sessions().is_empty() {
            monitor.poll().await;
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("T691/T1387: saved network never attached");
}

#[tokio::test]
async fn t1387_dropped_poll_and_network_retain_original_creator() {
    use futures_util::FutureExt;
    let (adb, mut network, _root) = fixture();
    assert!(network.devices(&adb, &[]).now_or_never().is_none());
    observed(&adb.0 .0.started).await;
    drop(network);
    tokio::task::yield_now().await;
    assert_eq!(adb.0 .0.settled.load(Ordering::SeqCst), 0);
    adb.0 .0.release.notify_one();
    observed(&adb.0 .0.retired).await;
    observed(&adb.0 .0.settled).await;
    assert!(!adb.0 .0.connected.load(Ordering::SeqCst));
}
