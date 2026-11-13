use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Counts {
    started: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
}
struct Query(Arc<Counts>);
impl Query {
    fn start(counts: &Arc<Counts>) -> Self {
        counts.started.fetch_add(1, Ordering::SeqCst);
        let active = counts.active.fetch_add(1, Ordering::SeqCst) + 1;
        counts.peak.fetch_max(active, Ordering::SeqCst);
        Self(counts.clone())
    }
}
impl Drop for Query {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}
#[derive(Clone)]
struct Delayed {
    fake: Fake,
    counts: Arc<Counts>,
    slow: Arc<Mutex<std::collections::BTreeSet<String>>>,
}
impl Commands for Delayed {
    fn execute(&self, args: Vec<String>, input: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            if args.get(3).map(String::as_str) != Some("getprop") {
                return self.fake.execute(args, input).await;
            }
            let _query = Query::start(&self.counts);
            let slow = self.slow.lock().unwrap().contains(&args[1]);
            let output = self.fake.execute(args, input).await?;
            if slow {
                tokio::time::sleep(Duration::from_secs(4)).await;
            }
            Ok(output)
        })
    }
}
fn probe_fixture(serials: &[&str]) -> (Adb<Delayed>, network::Network, tempfile::TempDir) {
    let (adb, _) = fixture();
    {
        let mut state = adb.0 .0.lock().unwrap();
        state.inventory = format!(
            "List of devices attached\n{}",
            serials
                .iter()
                .map(|s| format!("{s}\tdevice\n"))
                .collect::<String>()
        );
        state.identities = serials
            .iter()
            .map(|s| (s.to_string(), format!("identity-{s}")))
            .collect();
    }
    let delayed = Delayed {
        fake: adb.0,
        counts: Default::default(),
        slow: Arc::new(Mutex::new(
            serials
                .iter()
                .filter(|s| s.starts_with("slow"))
                .map(|s| s.to_string())
                .collect(),
        )),
    };
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    (Adb(delayed), network::Network::new(store), root)
}

#[tokio::test(start_paused = true)]
async fn t1381_healthy_device_progresses_before_slow_identity_queries_finish() {
    let (adb, mut network, _root) = probe_fixture(&["slow-a", "slow-b", "USB"]);
    let started = tokio::time::Instant::now();
    // Keep reconnect I/O outside this paused-clock identity scheduling test.
    let observed = tokio::time::timeout(
        Duration::from_secs(6),
        network.devices(&adb, &["slow-a".into()]),
    )
    .await;
    assert!(
        observed.is_ok(),
        "T1381: unrelated slow queries discarded discovery"
    );
    assert_eq!(observed.unwrap().unwrap(), ["USB"]);
    assert!(started.elapsed() < Duration::from_secs(4));
    assert_eq!(
        network.identity("USB").map(String::as_str),
        Some("identity-USB")
    );
    tokio::time::advance(Duration::from_secs(4)).await;
    tokio::task::yield_now().await;
    let observed = network.devices(&adb, &["USB".into()]).await.unwrap();
    assert_eq!(observed.len(), 3);
    assert!(adb.0.counts.peak.load(Ordering::SeqCst) <= 4);
}

#[tokio::test(start_paused = true)]
async fn t1381_cancelled_poll_retains_bounded_original_queries() {
    use futures_util::FutureExt;
    let (adb, mut network, _root) = probe_fixture(&["slow-a", "slow-b", "slow-c", "slow-d", "USB"]);
    assert!(network
        .devices(&adb, &["slow-a".into()])
        .now_or_never()
        .is_none());
    tokio::task::yield_now().await;
    let started = adb.0.counts.started.load(Ordering::SeqCst);
    assert_eq!(started, 4, "T1381: missing bounded per-device work");
    for _ in 0..4 {
        network.devices(&adb, &["slow-a".into()]).await.unwrap();
    }
    assert_eq!(adb.0.counts.started.load(Ordering::SeqCst), started);
    tokio::time::advance(Duration::from_secs(4)).await;
    for _ in 0..3 {
        network.devices(&adb, &["slow-a".into()]).await.unwrap();
    }
    assert!(
        network.identity("USB").is_some(),
        "T1381: queued device starved"
    );
    assert!(adb.0.counts.peak.load(Ordering::SeqCst) <= 4);
    drop(network);
    tokio::task::yield_now().await;
    assert_eq!(adb.0.counts.active.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn t1381_removed_query_cannot_publish_into_reused_endpoint() {
    use futures_util::FutureExt;
    let (adb, mut network, _root) = probe_fixture(&["slow-a"]);
    assert!(network
        .devices(&adb, &["slow-a".into()])
        .now_or_never()
        .is_none());
    tokio::task::yield_now().await;
    adb.0.fake.0.lock().unwrap().inventory = "List of devices attached\n".into();
    assert!(network.devices(&adb, &[]).await.unwrap().is_empty());
    {
        let mut state = adb.0.fake.0.lock().unwrap();
        state.inventory = "List of devices attached\nslow-a\tdevice\n".into();
        state
            .identities
            .insert("slow-a".into(), "replacement".into());
    }
    adb.0.slow.lock().unwrap().clear();
    for _ in 0..3 {
        network.devices(&adb, &[]).await.unwrap();
    }
    assert_eq!(
        network.identity("slow-a").map(String::as_str),
        Some("replacement")
    );
    assert_eq!(adb.0.counts.active.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn t1381_monitor_attaches_healthy_device_and_observes_its_removal() {
    let (adb, _network, root) = probe_fixture(&["slow-a", "slow-b", "USB"]);
    let (_, _, stop, ports) = monitor_fixture().await;
    let config = blent_config::FileConfig {
        video_port: ports.0,
        input_port: ports.1,
        ..Default::default()
    };
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    let mut monitor = monitor::Monitor::new(adb.clone(), config, stop.subscribe())
        .unwrap()
        .with_network(store);
    monitor.poll().await;
    let attached = monitor.sessions();
    adb.0.fake.0.lock().unwrap().inventory =
        "List of devices attached\nslow-a\tdevice\nslow-b\tdevice\n".into();
    monitor.poll().await;
    let removed = monitor.sessions().is_empty();
    monitor.shutdown().await.unwrap();
    assert_eq!(attached.len(), 1, "T1381: healthy attachment starved");
    assert_eq!(attached[0].identity.adb_serial(), Some("USB"));
    assert!(removed, "T1381: slow probes hid a confirmed device removal");
    assert_eq!(adb.0.counts.active.load(Ordering::SeqCst), 0);
}
