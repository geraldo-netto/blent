use super::super::{async_io::AsyncSocket, Socket};
use super::*;
use blent_config::connection::{
    control::Notice,
    management::{Key, Resource},
    pairing::Ticket,
};
use std::{
    net::TcpListener,
    os::{fd::OwnedFd, unix::fs::PermissionsExt},
};

async fn bounded<T>(work: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), work)
        .await
        .unwrap()
}
fn peer(value: u64) -> PeerId {
    PeerId::parse(format!("{value:064x}")).unwrap()
}
fn candidate() -> Candidate {
    Candidate {
        target: accessory::Target::new(1, &[2]).unwrap(),
        address: 3,
        vendor: 0x1234,
        product: 7,
        device_class: 0,
        support: Support::Unknown,
    }
}
struct Fixture {
    server: Server,
    _root: tempfile::TempDir,
}
impl Fixture {
    async fn new() -> Self {
        Self::configured(|_| {}).await
    }
    async fn configured(change: impl FnOnce(&mut Settings)) -> Self {
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("helper");
        std::fs::write(&helper, b"#!/usr/bin/python3\nimport sys\nif sys.argv[1] != '--list':\n    sys.exit(7)\nsys.stdout.buffer.write(bytes.fromhex('424c554900000001000100000000000001030100123400070200000000000000'))\n").unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut options = Settings::new(
            root.path().join("runtime"),
            root.path().join("state"),
            helper,
        );
        change(&mut options);
        let server = bounded(Server::open(options)).await.unwrap();
        Self {
            server,
            _root: root,
        }
    }
    async fn client(&mut self) -> (u64, AsyncSocket) {
        let remote = AsyncSocket::new(
            Socket::connect(&self.server.settings.runtime.join("broker.sock"), unsafe {
                libc::geteuid()
            })
            .unwrap(),
        )
        .unwrap();
        let (socket, _) = bounded(self.server.listener.accept()).await.unwrap();
        self.server.accept(socket).unwrap();
        let id = *self.server.clients.keys().next_back().unwrap();
        (id, remote)
    }
    async fn finish(self) {
        let completion = self.server.authority.completion();
        drop(self.server);
        bounded(async {
            while !completion.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await;
    }
}

mod policy;
mod registration;
mod runtime;
mod shutdown;
mod tls;
fn lease(owner: u64, resource: Resource) -> Lease {
    Lease {
        connection: Ticket {
            epoch: 1,
            peer: peer(1),
        },
        key: Key {
            resource,
            request: owner,
            generation: 1,
        },
        owner,
    }
}
fn registration(
    server: &mut Server,
    client: u64,
    owner: u64,
    resource: Resource,
    active: bool,
) -> RouteId {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut endpoints = Endpoints::validate(resource, vec![OwnedFd::from(listener)]).unwrap();
    let route = endpoints.reserve(&mut server.claims, owner).unwrap();
    server.routes.insert(
        owner,
        Registration {
            client,
            request: owner,
            connection: 1,
            route,
            endpoints,
            lease: active.then(|| lease(owner, resource)),
            closing: false,
        },
    );
    route
}
fn fill_replies(server: &Server, client: u64) {
    let peer = server.clients[&client].peer.as_ref().unwrap();
    for request in [100, 101] {
        peer.reply(vec![Message::Rejected {
            request,
            reason: Rejection::Busy,
        }])
        .unwrap();
    }
}
async fn response(remote: &AsyncSocket) -> Message {
    bounded(remote.receive()).await.unwrap().message
}
fn control(server: &mut Server, client: u64, request: u64, command: Command) {
    server
        .request(
            client,
            Received {
                message: Message::Control { request, command },
                descriptors: vec![],
            },
        )
        .unwrap();
}

#[tokio::test]
async fn t853_retired_route_reply_failure_still_cancels_every_client_sibling() {
    let mut fixture = Fixture::new().await;
    let (client, _remote) = fixture.client().await;
    registration(&mut fixture.server, client, 1, Resource::Camera, true);
    registration(&mut fixture.server, client, 2, Resource::Speakers, false);
    fill_replies(&fixture.server, client);
    fixture
        .server
        .route_state(1, lease(1, Resource::Camera), session::State::Retired)
        .unwrap();
    assert!(!fixture.server.routes.contains_key(&1));
    assert!(fixture.server.clients[&client].peer.is_none());
    assert!(
        fixture.server.routes[&2].closing,
        "retired route reentered cancellation and stranded sibling authority"
    );
    fixture.finish().await;
}

#[tokio::test]
async fn t853_failed_inventory_rejects_inspect_until_successful_refresh() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    fixture
        .server
        .inventory_result(Ok(vec![candidate()]))
        .unwrap();
    fixture
        .server
        .inventory_result(Err(anyhow::anyhow!("injected scan failure")))
        .unwrap();
    control(&mut fixture.server, client, 1, Command::Inspect);
    assert!(
        matches!(
            response(&remote).await,
            Message::Rejected { request: 1, .. }
        ),
        "scan failure was reported as successful empty inventory"
    );
    fixture.server.inventory_result(Ok(vec![])).unwrap();
    control(&mut fixture.server, client, 2, Command::Inspect);
    assert!(matches!(
        response(&remote).await,
        Message::Report {
            request: 2,
            notice: Notice::Begin { .. }
        }
    ));
    assert!(matches!(
        response(&remote).await,
        Message::Report {
            request: 2,
            notice: Notice::End { .. }
        }
    ));
    fixture.finish().await;
}
