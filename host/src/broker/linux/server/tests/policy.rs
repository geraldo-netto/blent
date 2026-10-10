use super::*;
use blent_config::connection::management::{Credential, Profile};
use futures_util::FutureExt;

#[tokio::test]
async fn t853_server_settings_validate_all_capacity_and_path_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let good = Settings::new(
        root.path().join("runtime"),
        root.path().join("state"),
        root.path().join("helper"),
    );
    good.validate().unwrap();
    for mode in 0..10 {
        let mut bad = good.clone();
        invalidate(&mut bad, mode);
        assert!(bad.validate().is_err());
    }
    let unsafe_path = root.path().join("unsafe");
    std::fs::create_dir(&unsafe_path).unwrap();
    std::fs::set_permissions(&unsafe_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut bad = good;
    bad.runtime = unsafe_path;
    assert!(bounded(Server::open(bad)).await.is_err());
}
fn invalidate(settings: &mut Settings, mode: u8) {
    match mode {
        0 => settings.runtime = "relative".into(),
        1 => settings.state = "relative".into(),
        2 => settings.helper = "relative".into(),
        3 => settings.trust_peers = 0,
        4 => settings.trust_peers = 1025,
        _ => invalid_capacity(settings, mode),
    }
}
fn invalid_capacity(settings: &mut Settings, mode: u8) {
    match mode {
        5 => settings.registrations = 0,
        6 => settings.block = 0,
        7 => settings.control.links = 0,
        8 => settings.session.channel_bytes = 0,
        _ => settings.session.queued_messages = 0,
    }
}

#[tokio::test]
async fn t853_server_local_client_retirement_preserves_capacity_until_task_exit() {
    let mut fixture = Fixture::configured(|settings| settings.control.clients = 1).await;
    let (id, first) = fixture.client().await;
    let second = AsyncSocket::new(
        Socket::connect(
            &fixture.server.settings.runtime.join("broker.sock"),
            unsafe { libc::geteuid() },
        )
        .unwrap(),
    )
    .unwrap();
    let (socket, _) = bounded(fixture.server.listener.accept()).await.unwrap();
    fixture.server.accept(socket).unwrap();
    assert_eq!(fixture.server.clients.len(), 1);
    assert!(bounded(second.receive()).await.is_err());
    first.close();
    let input = bounded(next(&mut fixture.server.clients, &mut fixture.server.links)).await;
    fixture.server.input(input).unwrap();
    bounded(async {
        loop {
            fixture.server.collect_clients();
            if fixture.server.clients.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(fixture
        .server
        .controller
        .command(id, Command::Inspect, 0)
        .is_err());
    assert!(next(&mut fixture.server.clients, &mut fixture.server.links)
        .now_or_never()
        .is_none());
    fixture.finish().await;
}

#[tokio::test]
async fn t853_server_rejects_client_replies_unknown_routes_and_unproven_registration() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    let incoming = Message::Rejected {
        request: 1,
        reason: Rejection::Busy,
    };
    remote.send(&incoming, &[]).await.unwrap();
    let input = bounded(next(&mut fixture.server.clients, &mut fixture.server.links)).await;
    fixture.server.input(input).unwrap();
    assert!(matches!(
        response(&remote).await,
        Message::Rejected { request: 1, .. }
    ));
    let unknown = RouteId {
        owner: 999,
        revision: 1,
    };
    fixture
        .server
        .request(
            client,
            Received {
                message: Message::Unregister {
                    request: 2,
                    route: unknown,
                },
                descriptors: vec![],
            },
        )
        .unwrap();
    assert!(matches!(
        response(&remote).await,
        Message::Rejected { request: 2, .. }
    ));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    fixture
        .server
        .request(
            client,
            Received {
                message: Message::Register {
                    request: 3,
                    peer: peer(1),
                    resource: Resource::Camera,
                    credential: Credential::new([1; 32]),
                    profile: Profile::Display,
                },
                descriptors: vec![OwnedFd::from(listener)],
            },
        )
        .unwrap();
    assert!(matches!(
        response(&remote).await,
        Message::Rejected { request: 3, .. }
    ));
    assert!(TcpListener::bind(address).is_ok());
    fixture.finish().await;
}

#[tokio::test]
async fn t853_server_route_ownership_stale_callbacks_and_closing_are_exact() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    let route = registration(&mut fixture.server, client, 1, Resource::Camera, false);
    assert!(fixture.server.unregister(client + 1, 2, route).is_err());
    let stale = RouteId {
        revision: route.revision + 1,
        ..route
    };
    assert!(fixture.server.unregister(client, 2, stale).is_err());
    fixture.server.unregister(client, 7, route).unwrap();
    assert!(matches!(
        response(&remote).await,
        Message::Status {
            request: 7,
            state: RouteState::Closing,
            ..
        }
    ));
    assert!(fixture
        .server
        .registered(2, 1, Ok(lease(1, Resource::Camera)))
        .is_err());
    assert!(fixture
        .server
        .registered(1, 1, Ok(lease(2, Resource::Camera)))
        .is_err());
    fixture
        .server
        .registered(1, 1, Err(anyhow::anyhow!("worker refused")))
        .unwrap();
    assert!(matches!(
        response(&remote).await,
        Message::Status {
            state: RouteState::Failed,
            ..
        }
    ));
    assert!(matches!(
        response(&remote).await,
        Message::Status {
            state: RouteState::Retired,
            ..
        }
    ));
    assert!(fixture.server.routes.is_empty());
    assert!(fixture
        .server
        .route_state(1, lease(1, Resource::Camera), session::State::Retired)
        .is_err());
    fixture.finish().await;
}

#[tokio::test]
async fn t853_server_authority_revokes_acknowledge_only_completed_storage_job() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    control(
        &mut fixture.server,
        client,
        1,
        Command::Revoke { peer: peer(8) },
    );
    assert_eq!(fixture.server.revocations.len(), 1);
    control(
        &mut fixture.server,
        client,
        2,
        Command::Revoke { peer: peer(9) },
    );
    assert!(matches!(
        response(&remote).await,
        Message::Rejected { request: 2, .. }
    ));
    fixture.server.authorize_next().unwrap();
    assert!(fixture.server.authority_busy);
    fixture.server.authorize_next().unwrap();
    let event = bounded(fixture.server.authority.next()).await.unwrap();
    fixture.server.authority_event(event).unwrap();
    assert!(!fixture.server.authority_busy);
    assert!(fixture.server.revocations.is_empty());
    assert!(matches!(
        response(&remote).await,
        Message::Report {
            request: 1,
            notice: Notice::Begin { .. }
        }
    ));
    assert!(matches!(
        response(&remote).await,
        Message::Report {
            request: 1,
            notice: Notice::End { .. }
        }
    ));
    fixture.server.authorize_next().unwrap();
    fixture.finish().await;
}
