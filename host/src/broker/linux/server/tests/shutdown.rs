use super::*;
use blent_config::retirement::Observe;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

struct Held(Arc<AtomicBool>);
impl Observe for Held {
    fn poll_complete(&mut self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
struct Release(Arc<AtomicBool>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[tokio::test]
async fn t853_shutdown_invalid_budgets_leave_live_clients_and_authority_intact() {
    let mut fixture = Fixture::new().await;
    let (client, _remote) = fixture.client().await;
    let completion = fixture.server.authority.completion();
    for budget in [
        Duration::ZERO,
        Duration::from_secs(60) + Duration::from_nanos(1),
        Duration::MAX,
    ] {
        assert!(fixture.server.shutdown(budget).await.is_err());
        assert!(fixture.server.clients[&client].peer.is_some());
        assert!(!completion.is_complete());
    }
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    assert!(fixture.server.clients.is_empty());
    assert!(completion.is_complete());
    fixture.finish().await;
}

#[tokio::test]
async fn t853_shutdown_live_authenticated_link_observes_task_native_and_authority_completion() {
    let mut auth = tls::authenticated().await;
    let server = &mut auth.fixture.server;
    let native = server.links[&auth.id]
        .handle
        .as_ref()
        .unwrap()
        .native_completion();
    let task = server.links[&auth.id].completion.clone();
    let authority = server.authority.completion();
    server.shutdown(Duration::from_secs(2)).await.unwrap();
    assert!(native.is_complete());
    assert!(task.is_complete());
    assert!(authority.is_complete());
    assert!(server.links.is_empty());
    assert!(server.clients.is_empty());
    assert!(server.controller.closing().is_empty());
    server.shutdown(Duration::from_secs(2)).await.unwrap();
    auth.fixture.finish().await;
}
fn completed_task() -> Completion {
    let admission = Pool::new(1).unwrap().reserve().unwrap();
    let completion = admission.completion();
    drop(admission);
    completion
}

#[tokio::test]
async fn t853_shutdown_timeout_keeps_native_quarantine_until_actual_completion_and_retry() {
    let mut fixture = Fixture::configured(|settings| settings.control.links = 1).await;
    let (client, _remote) = fixture.client().await;
    let server = &mut fixture.server;
    let id = tls::selection(server, client);
    let admission = server.link_pool.reserve().unwrap();
    let completion = admission.completion();
    let release = Release(Arc::new(AtomicBool::new(false)));
    admission.retain(Held(release.0.clone()));
    server.controller.attach(id, completion.clone()).unwrap();
    server.links.insert(
        id,
        Link {
            handle: None,
            completion: completed_task(),
            verified: None,
        },
    );
    assert!(server.shutdown(Duration::from_millis(20)).await.is_err());
    assert_eq!(server.controller.closing(), vec![id]);
    assert!(!completion.is_complete());
    assert!(!server.authority.completion().is_complete());
    assert_eq!(server.link_pool.active(), 1);
    assert!(server.link_pool.reserve().is_err());
    drop(release);
    server.shutdown(Duration::from_secs(2)).await.unwrap();
    assert!(completion.is_complete());
    assert_eq!(server.link_pool.active(), 0);
    assert!(server.controller.closing().is_empty());
    assert!(server.authority.completion().is_complete());
    fixture.finish().await;
}

#[tokio::test]
async fn t853_shutdown_drains_started_inventory_and_queued_durable_revocation() {
    let mut fixture = Fixture::new().await;
    let (client, _remote) = fixture.client().await;
    fixture.server.scan().unwrap();
    control(
        &mut fixture.server,
        client,
        1,
        Command::Revoke { peer: peer(8) },
    );
    fixture.server.authorize_next().unwrap();
    assert!(fixture.server.authority_busy);
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    assert!(!fixture.server.authority_busy);
    assert!(fixture.server.inventory.as_ref().unwrap().retired());
    assert!(fixture.server.authority.completion().is_complete());
    assert!(fixture.server.controller.closing().is_empty());
    fixture.finish().await;
}
