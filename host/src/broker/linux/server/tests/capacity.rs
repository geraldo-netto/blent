//! T1600: overload rejects before route ownership and keeps admitted clients live.
use super::*;

#[tokio::test]
async fn t1600_excess_client_receives_request_bound_busy_without_disrupting_owner() {
    let mut fixture = Fixture::configured(|settings| settings.control.clients = 1).await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(60);
    let (id, admitted) = fixture.client().await;
    let rejected = AsyncSocket::new(
        Socket::connect(
            &fixture.server.settings.runtime.join("broker.sock"),
            unsafe { libc::geteuid() },
        )
        .unwrap(),
    )
    .unwrap();
    rejected
        .send(
            &Message::Control {
                request: 37,
                command: Command::Inspect,
            },
            &[],
        )
        .await
        .unwrap();
    let (socket, _) = bounded(fixture.server.listener.accept()).await.unwrap();
    fixture.server.accept(socket).unwrap();
    let (running, response) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(100), fixture.server.run()),
        tokio::time::timeout(Duration::from_millis(80), rejected.receive()),
    );
    assert!(running.is_err(), "T1600 full broker must stay running");
    assert!(
        matches!(
            response,
            Ok(Ok(Received {
                message: Message::Rejected {
                    request: 37,
                    reason: Rejection::Busy,
                },
                ..
            }))
        ),
        "T1600 overload did not send exact Busy"
    );
    assert!(fixture.server.clients.contains_key(&id));
    assert!(fixture.server.routes.is_empty());
    admitted.close();
    rejected.close();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}

#[test]
fn t1600_default_clients_cover_registration_capacity_and_control_headroom() {
    let settings = Settings::new("/runtime".into(), "/state".into(), "/helper".into());
    assert!(settings.control.clients >= settings.registrations + 4);
}

#[tokio::test]
async fn t1600_registration_headroom_is_funded_by_actual_client_admission() {
    let mut fixture = Fixture::configured(|settings| {
        settings.control.clients = settings.registrations + 4;
    })
    .await;
    let count = fixture.server.settings.registrations + 4;
    let mut remotes = Vec::new();
    for _ in 0..count {
        remotes.push(fixture.client().await.1);
    }
    let admitted = fixture.server.clients.len();
    for remote in remotes {
        remote.close();
    }
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
    assert_eq!(
        admitted, count,
        "T1600 advertised capacity has an unfunded inner task limit"
    );
}

fn excess(server: &Server) -> AsyncSocket {
    AsyncSocket::new(
        Socket::connect(&server.settings.runtime.join("broker.sock"), unsafe {
            libc::geteuid()
        })
        .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn t1600_silent_excess_peer_cannot_stall_admitted_control_or_shutdown() {
    let mut fixture = Fixture::configured(|settings| {
        settings.control.clients = 1;
        settings.session.io_timeout = Duration::from_millis(100);
    })
    .await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(60);
    fixture.server.inventory_observed = true;
    let (_, admitted) = fixture.client().await;
    let silent = excess(&fixture.server);
    let (socket, _) = bounded(fixture.server.listener.accept()).await.unwrap();
    fixture.server.accept(socket).unwrap();
    assert!(fixture.server.rejector.active());
    admitted
        .send(
            &Message::Control {
                request: 49,
                command: Command::Inspect,
            },
            &[],
        )
        .await
        .unwrap();
    let (_, response) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(200), fixture.server.run()),
        tokio::time::timeout(Duration::from_millis(80), admitted.receive()),
    );
    assert_eq!(response.unwrap().unwrap().message.request(), 49);
    assert!(!fixture.server.rejector.active());
    assert!(bounded(silent.receive()).await.is_err());
    let pending = excess(&fixture.server);
    let (socket, _) = bounded(fixture.server.listener.accept()).await.unwrap();
    fixture.server.accept(socket).unwrap();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    assert!(!fixture.server.rejector.active());
    assert!(bounded(pending.receive()).await.is_err());
    fixture.finish().await;
}

fn registration_factory(server: &Server) -> super::super::super::registration_client::Factory {
    use super::super::super::{
        client::Launch,
        registration_client::{Factory, Settings},
    };
    let mut settings = Settings::new(Launch {
        executable: "/bin/false".into(),
        runtime: server.settings.runtime.clone(),
        state: server.settings.state.clone(),
        helper: server.settings.helper.clone(),
    });
    settings.capacity = 1;
    Factory::new(settings).unwrap()
}

#[tokio::test]
async fn t1600_busy_settles_real_registration_and_reuses_original_slot() {
    use blent_config::connection::{
        management::{Credential, Profile},
        registration_client::{Client, Failure, Request},
    };
    let mut fixture = Fixture::configured(|settings| settings.control.clients = 1).await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(60);
    let (_, admitted) = fixture.client().await;
    let factory = registration_factory(&fixture.server);
    for _ in 0..3 {
        let request = Request {
            peer: peer(7),
            resource: Resource::DisplaySession,
            credential: Credential::new([9; 32]),
            profile: Profile::Display,
        };
        let listeners: Vec<_> = (0..2)
            .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
            .collect();
        let addresses: Vec<_> = listeners
            .iter()
            .map(|listener| listener.local_addr().unwrap())
            .collect();
        let client = factory.start(request, listeners).unwrap();
        let (_, ()) = tokio::join!(
            tokio::time::timeout(Duration::from_millis(250), fixture.server.run()),
            bounded(async {
                while !client.retired() {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            }),
        );
        for address in addresses {
            assert!(
                TcpListener::bind(address).is_ok(),
                "T1600 retained listener after exact Busy retirement"
            );
        }
        assert!(client.status().remote_retired);
        assert_eq!(
            client.status().failure,
            Some(Failure::Rejected(Rejection::Busy))
        );
        assert!(fixture.server.routes.is_empty());
    }
    admitted.close();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn t1600_pending_client_retirement_retains_admission_and_rejects_retry() {
    let mut fixture = Fixture::configured(|settings| settings.control.clients = 1).await;
    let retained = fixture.server.client_pool.reserve().unwrap();
    let remote = excess(&fixture.server);
    remote
        .send(
            &Message::Control {
                request: 71,
                command: Command::Inspect,
            },
            &[],
        )
        .await
        .unwrap();
    let (socket, _) = bounded(fixture.server.listener.accept()).await.unwrap();
    fixture.server.accept(socket).unwrap();
    assert!(fixture.server.clients.is_empty());
    bounded(fixture.server.rejector.next()).await;
    assert!(matches!(
        bounded(remote.receive()).await.unwrap().message,
        Message::Rejected {
            request: 71,
            reason: Rejection::Busy
        }
    ));
    drop(retained);
    let (_, admitted) = fixture.client().await;
    assert_eq!(fixture.server.clients.len(), 1);
    admitted.close();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}
