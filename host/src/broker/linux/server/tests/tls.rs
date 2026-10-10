use super::*;
mod ambiguity;
use crate::{
    accessory::{setup::tests::remote_hello, Owner, Resource as NativeResource, Stream},
    secure_connection::{self, Identity, IdentityRecord, PeerCertificate},
};
use blent_config::{
    connection::{ready, window},
    retirement::Admission,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Native(Admission);
impl Owner for Native {
    fn completion(&self) -> Completion {
        self.0.completion()
    }
    fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}
pub(super) struct Authenticated {
    pub fixture: Fixture,
    pub remote: AsyncSocket,
    pub client: u64,
    pub id: u64,
    pub tablet: Box<dyn Stream>,
    pub pin: PeerId,
}
pub(super) fn selection(server: &mut Server, client: u64) -> u64 {
    server.inventory_result(Ok(vec![candidate()])).unwrap();
    let (revision, target) = server
        .controller
        .snapshot()
        .iter()
        .find_map(|item| match item {
            Notice::Device {
                revision, target, ..
            } => Some((*revision, *target)),
            _ => None,
        })
        .unwrap();
    match server
        .controller
        .command(client, Command::Connect { revision, target }, 0)
        .unwrap()
    {
        Action::Start { connection, .. } => connection,
        _ => panic!("no connection"),
    }
}
fn attach(server: &mut Server, id: u64) -> tokio::io::DuplexStream {
    let (stream, peer) = tokio::io::duplex(8192);
    let resource = NativeResource {
        stream: Box::new(stream),
        owner: Box::new(Native(server.link_pool.reserve().unwrap())),
    };
    let handle = Handle::start(
        resource,
        link::Settings {
            identity: server.identity.identity.clone(),
            limits: server.settings.session,
            block: server.settings.block,
            setup_timeout: Duration::from_secs(2),
            approval_timeout: Duration::from_secs(120),
            origin: server.origin,
        },
    )
    .unwrap();
    server
        .controller
        .attach(id, handle.native_completion())
        .unwrap();
    server.links.insert(
        id,
        Link {
            completion: handle.task_completion(),
            handle: Some(handle),
            verified: None,
        },
    );
    peer
}
async fn event(server: &mut Server, id: u64) -> link::Event {
    bounded(
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
    .unwrap()
}
async fn remote_tls(
    mut remote: tokio::io::DuplexStream,
    tablet: Identity,
    host: Arc<Identity>,
) -> Box<dyn Stream> {
    let mut marker = [0; ready::BYTES];
    remote.read_exact(&mut marker).await.unwrap();
    ready::decode(&marker, true).unwrap();
    remote.write_all(&ready::encode(false)).await.unwrap();
    let host = PeerCertificate::parse(host.certificate().to_vec()).unwrap();
    let mut stream = secure_connection::connect(remote, &tablet, &host)
        .await
        .unwrap();
    let mut bytes = [0; window::BYTES];
    stream.read_exact(&mut bytes).await.unwrap();
    assert_eq!(window::decode(&bytes, true).unwrap(), 4096);
    stream
        .write_all(&window::encode(false, 4096).unwrap())
        .await
        .unwrap();
    stream.flush().await.unwrap();
    Box::new(stream)
}
fn approve(server: &mut Server, client: u64, id: u64) -> PeerId {
    let ticket = server
        .controller
        .snapshot()
        .iter()
        .find_map(|notice| match notice {
            Notice::Link { connection, ticket, .. } if *connection == id => ticket.clone(),
            _ => None,
        })
        .unwrap();
    let peer = ticket.peer.clone();
    let action = server
        .controller
        .command(
            client,
            Command::Decide {
                connection: id,
                ticket,
                allow: true,
                remember: true,
            },
            link::now(server.origin).unwrap(),
        )
        .unwrap();
    server.action(action).unwrap();
    peer
}
pub(super) async fn authenticated() -> Authenticated {
    let mut fixture = Fixture::configured(|settings| settings.session.channel_bytes = 4096).await;
    let (client, remote) = fixture.client().await;
    let server = &mut fixture.server;
    let id = selection(server, client);
    let mut stream = attach(server, id);
    let tablet = Identity::load(IdentityRecord::generate().unwrap()).unwrap();
    bounded(remote_hello(&mut stream, tablet.certificate())).await;
    let identified = event(server, id).await;
    server.link_event(id, identified).unwrap();
    let pin = approve(server, client, id);
    let host = server.identity.identity.clone();
    let (verified, tablet) =
        bounded(async { tokio::join!(event(server, id), remote_tls(stream, tablet, host)) }).await;
    server.link_event(id, verified).unwrap();
    server.authorize_next().unwrap();
    let completed = bounded(server.authority.next()).await.unwrap();
    server.authority_event(completed).unwrap();
    assert!(server.controller.lease(id).is_some());
    assert!(server
        .controller
        .snapshot()
        .iter()
        .any(|notice| matches!(notice, Notice::Saved { peer, .. } if peer == &pin)));
    Authenticated {
        fixture,
        remote,
        client,
        id,
        tablet,
        pin,
    }
}
