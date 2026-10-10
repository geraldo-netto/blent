use super::*;
use blent_config::input_mapping::{Rect, Rotation};
use std::sync::atomic::{AtomicUsize, Ordering};
fn snapshot(id: &str) -> Snapshot {
    Snapshot::new(vec![Monitor {
        id: id.into(),
        name: id.into(),
        bounds: Rect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        },
        rotation: Rotation::Identity,
        scale_percent: 100,
        primary: true,
    }])
    .unwrap()
}
struct Held {
    calls: Arc<AtomicUsize>,
    entered: mpsc::Sender<()>,
    replies: Mutex<mpsc::Receiver<Result<Snapshot, String>>>,
}
impl MonitorInventory for Held {
    fn snapshot(&self) -> blent_config::input_mapping::InventoryResult {
        self.calls.fetch_add(1, Ordering::AcqRel);
        self.entered.send(()).unwrap();
        self.replies
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .map_err(|e| std::io::Error::other(e).into())
    }
}
fn wait(mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(2);
    while !condition() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn t825_held_inventory_never_blocks_redraw_and_retains_capacity_until_completion() {
    let pool = Pool::new(1).unwrap();
    let published = Published::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered, starts) = mpsc::channel();
    let (send, replies) = mpsc::channel();
    let source = Held {
        calls: calls.clone(),
        entered,
        replies: Mutex::new(replies),
    };
    let repaint = Arc::new(AtomicUsize::new(0));
    let painted = repaint.clone();
    let worker = Worker::with_pool(
        &pool,
        published.clone(),
        source,
        move || {
            painted.fetch_add(1, Ordering::AcqRel);
        },
        Duration::from_millis(5),
    )
    .unwrap();
    starts.recv_timeout(Duration::from_secs(2)).unwrap();
    for _ in 0..1000 {
        assert!(published
            .read()
            .notice(Instant::now())
            .unwrap()
            .contains("waiting"));
    }
    assert_eq!(calls.load(Ordering::Acquire), 1);
    send.send(Ok(snapshot("first"))).unwrap();
    wait(|| repaint.load(Ordering::Acquire) == 1);
    assert_eq!(published.read().monitors()[0].id, "first");
    starts.recv_timeout(Duration::from_secs(2)).unwrap();
    send.send(Ok(snapshot("replacement"))).unwrap();
    wait(|| repaint.load(Ordering::Acquire) == 2);
    assert_eq!(published.read().monitors()[0].id, "replacement");
    starts.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(worker);
    assert_eq!(pool.active(), 1);
    assert!(pool.reserve().is_err());
    send.send(Ok(snapshot("retired"))).unwrap();
    wait(|| pool.active() == 0);
    assert_eq!(repaint.load(Ordering::Acquire), 2);
    assert_eq!(published.read().monitors()[0].id, "replacement");
}
#[test]
fn t825_stale_failed_and_changed_snapshots_preserve_identity_until_recovery() {
    let published = Published::default();
    let start = Instant::now();
    assert!(published.read().monitors().is_empty());
    published.publish(Ok(snapshot("first")), start);
    let reading = published.read();
    assert!(reading.notice(start).is_none());
    assert!(reading.notice(start + Duration::from_secs(5)).is_none());
    assert!(reading
        .notice(start + Duration::from_secs(6))
        .unwrap()
        .contains("stale"));
    assert!(reading.notice(start - Duration::from_secs(1)).is_none());
    published.publish(Err("fixture failure".into()), start);
    assert!(published
        .read()
        .notice(start)
        .unwrap()
        .contains("fixture failure"));
    assert_eq!(published.read().monitors()[0].id, "first");
    published.publish(Ok(snapshot("second")), start);
    assert_eq!(published.read().monitors()[0].id, "second");
    assert!(published.read().notice(start).is_none());
    assert_eq!(reading.monitors()[0].id, "first");
}
struct Immediate;
impl MonitorInventory for Immediate {
    fn snapshot(&self) -> blent_config::input_mapping::InventoryResult {
        Ok(snapshot("ready"))
    }
}
#[test]
fn t825_default_worker_publishes_and_drop_wakes_idle_wait() {
    let (published, worker) = discover(Immediate, || {});
    let worker = worker.unwrap();
    let (unavailable, rejected) = discover(Immediate, || {});
    assert!(rejected.is_none());
    assert!(unavailable
        .read()
        .notice(Instant::now())
        .unwrap()
        .contains("unavailable"));
    wait(|| !published.read().monitors().is_empty());
    drop(worker);
}
