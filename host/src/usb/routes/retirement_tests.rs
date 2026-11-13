use super::*;

struct Lease(Arc<AtomicBool>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
#[test]
fn t1410_route_thread_exit_preserves_lease_until_original_native_session_settles() {
    let native = Pool::new(1).unwrap().reserve().unwrap();
    let released = Arc::new(AtomicBool::new(false));
    let mut retiring = Retiring {
        _report: watch::channel(None).0,
        thread: blent_config::retirement_work::Thread::new(std::thread::spawn(|| ())),
        control: Arc::new(Control {
            serial: Arc::new(Mutex::new("USB".into())),
            expected: [(8890, 9000), (8891, 9001)],
            reservation: Mutex::new(Some(Box::new(Lease(released.clone())))),
            settled: AtomicBool::new(true),
            native: Some(native.completion()),
        }),
    };
    let start = std::time::Instant::now();
    while !retiring.thread.poll_complete() {
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        !retiring.poll_complete(),
        "T1410 route cleanup substituted for native input cleanup"
    );
    assert!(!released.load(Ordering::Acquire));
    drop(native);
    assert!(retiring.poll_complete());
    drop(retiring);
    assert!(released.load(Ordering::Acquire));
}

#[tokio::test]
async fn t1410_connection_loss_keeps_original_native_lease_fence() {
    let (adb, connection) = super::super::tests::fixture();
    let native = Pool::new(1).unwrap().reserve().unwrap();
    let mut connection = connection
        .with_native_retirement(native.completion())
        .unwrap();
    connection.connect(&adb, "USB").await.unwrap();
    assert!(connection.disconnect(&adb).await.is_err());
    assert!(connection.serial().is_none());
    let routes = connection.release_routes().unwrap();
    let proof = routes.completion().unwrap();
    drop(connection);
    drop(routes);
    assert!(!proof.is_complete());
    drop(native);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1410_native_lifetime_cannot_rebind_after_admission() {
    let (adb, connection) = super::super::tests::fixture();
    let native = Pool::new(1).unwrap().reserve().unwrap();
    let connection = connection
        .with_native_retirement(native.completion())
        .unwrap();
    assert!(connection
        .with_native_retirement(native.completion())
        .is_err());
    let mut routes = Routes::new("USB", (19000, 19001)).unwrap();
    routes.retain_native(native.completion()).unwrap();
    assert!(routes.retain_native(native.completion()).is_err());
    routes.prepare(&adb).await.unwrap();
    assert!(routes.retain_native(native.completion()).is_err());
    drop(native);
    routes.retire(&adb).await.unwrap();
}
