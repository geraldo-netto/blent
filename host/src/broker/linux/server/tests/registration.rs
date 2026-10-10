use super::tls::authenticated;
use super::*;
use blent_config::connection::management::{Credential, Profile};
use std::os::fd::AsFd;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) fn listeners() -> (TcpListener, TcpListener) {
    let video = TcpListener::bind("127.0.0.1:0").unwrap();
    let control = TcpListener::bind("127.0.0.1:0").unwrap();
    video.set_nonblocking(true).unwrap();
    control.set_nonblocking(true).unwrap();
    (video, control)
}
async fn active(remote: &AsyncSocket) -> RouteId {
    let mut registered = None;
    for _ in 0..8 {
        match response(remote).await {
            Message::Registered { request: 1, route } => registered = Some(route),
            Message::Status {
                request: 1,
                route,
                state: RouteState::Active,
            } => {
                assert_eq!(registered, Some(route));
                return route;
            }
            Message::Status {
                request: 1,
                state: RouteState::Queued,
                ..
            } => (),
            _ => panic!("unexpected registration response"),
        }
    }
    panic!("resource never admitted");
}
pub(super) async fn exchange(
    video: TcpListener,
    control: TcpListener,
) -> (tokio::net::TcpStream, tokio::net::TcpStream) {
    let video = tokio::net::TcpListener::from_std(video).unwrap();
    let control = tokio::net::TcpListener::from_std(control).unwrap();
    let (video, _) = video.accept().await.unwrap();
    let (mut control, _) = control.accept().await.unwrap();
    control.write_all(b"host control").await.unwrap();
    let mut bytes = [0; 14];
    control.read_exact(&mut bytes).await.unwrap();
    assert_eq!(&bytes, b"tablet control");
    (video, control)
}
#[tokio::test]
async fn t853_server_real_tls_remembered_registration_binds_local_descriptors_and_revokes() {
    let mut auth = authenticated().await;
    let (video, control) = listeners();
    auth.remote
        .send(
            &Message::Register {
                request: 1,
                peer: auth.pin.clone(),
                resource: Resource::DisplaySession,
                credential: Credential::new([9; 32]),
                profile: Profile::Display,
            },
            &[video.as_fd(), control.as_fd()],
        )
        .await
        .unwrap();
    let input = bounded(next(
        &mut auth.fixture.server.clients,
        &mut auth.fixture.server.links,
    ))
    .await;
    auth.fixture.server.input(input).unwrap();
    assert_eq!(auth.fixture.server.routes.len(), 1);
    let (stop, done) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        tokio::select! { result = auth.fixture.server.run() => panic!("server failed: {result:?}"), _ = done => () }
        auth.fixture
    });
    let tablet = tokio::spawn(crate::broker::session::tests::tls::tablet(auth.tablet));
    let (route, _sockets) =
        bounded(async { tokio::join!(active(&auth.remote), exchange(video, control)) }).await;
    let _tablet = bounded(tablet).await.unwrap();
    stop.send(()).unwrap();
    let mut fixture = bounded(running).await.unwrap();
    cancel_active(&mut fixture.server, auth.client, auth.id, route).await;
    control_revoke(&mut fixture.server, auth.client, auth.pin);
    let reply = bounded(fixture.server.authority.next()).await.unwrap();
    fixture.server.authority_event(reply).unwrap();
    fixture.server.close_requested().unwrap();
    bounded(async {
        while !fixture.server.links.is_empty() {
            fixture.server.collect_links().unwrap();
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(fixture.server.routes.is_empty());
    assert!(fixture.server.controller.lease(auth.id).is_none());
    assert!(!fixture
        .server
        .controller
        .snapshot()
        .iter()
        .any(|n| matches!(n, Notice::Saved { .. })));
    fixture.finish().await;
}
fn control_revoke(server: &mut Server, client: u64, peer: PeerId) {
    control(server, client, 2, Command::Revoke { peer });
    server.authorize_next().unwrap();
    assert!(server.authority_busy);
}

async fn cancel_active(server: &mut Server, client: u64, id: u64, route: RouteId) {
    let expected = server.routes[&route.owner].lease.clone().unwrap();
    let sibling = registration(server, client, 99, Resource::Speakers, false);
    server.routes.get_mut(&sibling.owner).unwrap().connection = id;
    server.unregister(client, 3, route).unwrap();
    assert!(server.routes[&route.owner].closing);
    assert!(!server.routes[&sibling.owner].closing);
    assert!(server.controller.lease(id).is_some());
    let event = bounded(
        server
            .links
            .get_mut(&id)
            .unwrap()
            .handle
            .as_mut()
            .unwrap()
            .next(),
    )
    .await
    .unwrap();
    match event {
        link::Event::State(actual, session::State::Closing) => assert_eq!(actual, expected),
        _ => panic!("exact resource cancellation was not delivered"),
    }
}

#[tokio::test]
async fn t853_active_route_cancel_with_closed_link_revokes_connection_authority() {
    let mut auth = authenticated().await;
    let route = registration(
        &mut auth.fixture.server,
        auth.client,
        88,
        Resource::Camera,
        true,
    );
    auth.fixture
        .server
        .routes
        .get_mut(&route.owner)
        .unwrap()
        .connection = auth.id;
    let handle = auth
        .fixture
        .server
        .links
        .get_mut(&auth.id)
        .unwrap()
        .handle
        .take()
        .unwrap();
    handle.stop();
    drop(handle);
    auth.fixture.server.cancel_route(route.owner).unwrap();
    assert!(auth.fixture.server.routes[&route.owner].closing);
    assert!(auth.fixture.server.controller.lease(auth.id).is_none());
    assert_eq!(auth.fixture.server.controller.closing(), vec![auth.id]);
    auth.fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    auth.fixture.finish().await;
}
