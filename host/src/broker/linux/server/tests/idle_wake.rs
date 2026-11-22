use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::task::{Context as TaskContext, Wake, Waker};

struct CountWake {
    count: Arc<AtomicUsize>,
    parent: Waker,
}
impl Wake for CountWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.parent.wake_by_ref();
    }
}

async fn observe_wakes(server: &mut Server, duration: Duration) -> usize {
    let count = Arc::new(AtomicUsize::new(0));
    let mut observer = None;
    let mut running = Box::pin(server.run());
    let measured = std::future::poll_fn(|cx| {
        let waker = observer.get_or_insert_with(|| {
            Waker::from(Arc::new(CountWake {
                count: count.clone(),
                parent: cx.waker().clone(),
            }))
        });
        running.as_mut().poll(&mut TaskContext::from_waker(waker))
    });
    tokio::select! {
        result = measured => panic!("unexpected broker exit: {result:?}"),
        _ = tokio::time::sleep(duration) => (),
    }
    count.load(Ordering::SeqCst)
}

#[tokio::test]
async fn t1776_empty_broker_has_no_periodic_dispatcher_wakeups() {
    let mut fixture = Fixture::new().await;
    let wakes = observe_wakes(&mut fixture.server, Duration::from_millis(600)).await;
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    println!("T1776 empty-broker dispatcher wakes in 600 ms: {wakes}");
    assert_eq!(
        wakes, 0,
        "T1776 quiescent broker kept a periodic dispatcher timer"
    );
}

async fn inspect_after(runtime: std::path::PathBuf, delay: Duration) -> Result<Vec<Notice>> {
    tokio::time::sleep(delay).await;
    let remote = AsyncSocket::new(Socket::connect(&runtime.join("broker.sock"), unsafe {
        libc::geteuid()
    })?)?;
    remote
        .send(
            &Message::Control {
                request: 17,
                command: Command::Inspect,
            },
            &[],
        )
        .await?;
    let mut notices = Vec::new();
    for _ in 0..1602 {
        let received = remote.receive().await?;
        let Message::Report { request, notice } = received.message else {
            anyhow::bail!("unexpected inspection reply")
        };
        ensure!(request == 17, "wrong inspection request");
        if matches!(notice, Notice::End { .. }) {
            return Ok(notices);
        }
        notices.push(notice);
    }
    anyhow::bail!("inspection exceeded bounded snapshot")
}

async fn resumed_inspection(mut fixture: Fixture, delay: Duration) {
    let runtime = fixture.server.settings.runtime.clone();
    let received = tokio::select! {
        result = fixture.server.run() => Err(anyhow::anyhow!("server exited: {result:?}")),
        result = tokio::time::timeout(Duration::from_secs(5), inspect_after(runtime, delay)) => result.map_err(anyhow::Error::from).and_then(|result| result),
    };
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    let received = received.unwrap();
    assert!(
        received
            .iter()
            .any(|notice| matches!(notice, Notice::Device { .. })),
        "T1776 returning client never received fresh inventory"
    );
}

#[tokio::test]
async fn t1776_new_socket_wakes_the_quiescent_dispatcher_and_discovers_devices() {
    resumed_inspection(Fixture::new().await, Duration::from_millis(60)).await;
}

#[tokio::test]
async fn t1776_delayed_accept_recovers_without_any_admitted_client() {
    let mut fixture = Fixture::new().await;
    fixture
        .server
        .accepted(Err(std::io::Error::from_raw_os_error(libc::EMFILE)))
        .unwrap();
    resumed_inspection(fixture, Duration::ZERO).await;
}

#[tokio::test]
async fn t1776_admitted_inventory_and_saved_peers_keep_maintenance_live() {
    let mut fixture = Fixture::new().await;
    assert!(fixture.server.maintenance_deadline().is_none());
    fixture.server.controller =
        Control::new(peer(0), vec![peer(8)], fixture.server.settings.control).unwrap();
    assert!(fixture.server.maintenance_deadline().is_some());
    fixture.server.scan().unwrap();
    fixture.server.controller =
        Control::new(peer(0), vec![], fixture.server.settings.control).unwrap();
    assert!(fixture.server.inventory.is_some());
    // Active inventory has a channel wake; only its finished native owner polls.
    assert!(fixture.server.maintenance_deadline().is_none());
    let result = bounded(inventory_next(&mut fixture.server.inventory)).await;
    fixture.server.inventory_result(result).unwrap();
    assert!(fixture.server.maintenance_deadline().is_some());
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.server.scan().unwrap();
    assert!(fixture.server.maintenance_deadline().is_none());
    fixture.finish().await;
}

#[tokio::test]
async fn t1777_live_client_waits_for_inventory_deadline_without_periodic_ticks() {
    let mut fixture = Fixture::new().await;
    let _client = fixture.client().await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(2);
    let wakes = observe_wakes(&mut fixture.server, Duration::from_millis(600)).await;
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    println!("T1777 live-client dispatcher wakes in 600 ms: {wakes}");
    assert_eq!(wakes, 0, "T1777 live idle client kept periodic ticks");
}

#[tokio::test]
async fn t1777_maintenance_selects_earliest_and_observes_only_retiring_clients() {
    let mut fixture = Fixture::new().await;
    let (client, _remote) = fixture.client().await;
    let now = Instant::now();
    fixture.server.next_inventory = now + Duration::from_secs(2);
    fixture.server.accept_after = now + Duration::from_secs(1);
    assert_eq!(
        fixture.server.maintenance_deadline(),
        Some(fixture.server.accept_after)
    );
    fixture.server.close_client(client).unwrap();
    fixture.server.next_observation = now;
    assert_eq!(fixture.server.maintenance_deadline(), Some(now));
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
}

fn pending_allocations(server: &mut Server) -> crate::allocation_probe::Counts {
    let mut context = TaskContext::from_waker(futures_util::task::noop_waker_ref());
    let mut running = Box::pin(server.run());
    assert!(running.as_mut().poll(&mut context).is_pending());
    let (_, counts) = crate::allocation_probe::measure(|| {
        for _ in 0..256 {
            assert!(running.as_mut().poll(&mut context).is_pending());
        }
    });
    counts
}

#[tokio::test]
async fn t1772_zero_link_dispatcher_has_no_steady_pending_allocations() {
    let mut fixture = Fixture::new().await;
    let counts = pending_allocations(&mut fixture.server);
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    assert_eq!(counts.allocations + counts.reallocations, 0);
    println!("T1772 zero links: 256 pending turns, 0 allocations/reallocations");
}

#[tokio::test]
async fn t1772_authenticated_idle_link_suspends_dispatcher_and_retires_exactly() {
    let mut authenticated = super::tls::authenticated().await;
    let server = &mut authenticated.fixture.server;
    server.next_inventory = Instant::now() + Duration::from_secs(2);
    assert_eq!(server.links.len(), 1);
    let completion = server.links[&authenticated.id].completion.clone();
    let counts = pending_allocations(server);
    let wakes = observe_wakes(server, Duration::from_millis(600)).await;
    assert!(!completion.is_complete());
    assert_eq!(server.link_pool.active(), 1);
    server.shutdown(Duration::from_secs(5)).await.unwrap();
    assert!(completion.is_complete());
    assert_eq!(server.link_pool.active(), 0);
    drop(authenticated.tablet);
    drop(authenticated.remote);
    authenticated.fixture.finish().await;
    println!("T1772 one authenticated link: wakes in 600 ms={wakes}, pending turns=256, allocations={}, reallocations={}", counts.allocations, counts.reallocations);
    assert_eq!(wakes, 0);
    assert_eq!(counts.allocations + counts.reallocations, 0);
}
