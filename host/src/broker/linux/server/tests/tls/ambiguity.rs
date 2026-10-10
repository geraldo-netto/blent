//! T921: distinct native TLS links can prove the same persisted certificate.
use super::*;
use blent_config::connection::{
    control::State,
    management::{Credential, Profile},
};

fn select(server: &mut Server, client: u64, index: usize) -> u64 {
    let targets: Vec<_> = server
        .controller
        .snapshot()
        .into_iter()
        .filter_map(|notice| match notice {
            Notice::Device {
                revision, target, ..
            } => Some((revision, target)),
            _ => None,
        })
        .collect();
    let (revision, target) = targets[index];
    match server
        .controller
        .command(client, Command::Connect { revision, target }, 0)
        .unwrap()
    {
        Action::Start { connection, .. } => connection,
        _ => panic!("no selected connection"),
    }
}
async fn authenticate(
    server: &mut Server,
    client: u64,
    index: usize,
    record: &IdentityRecord,
) -> (u64, Box<dyn Stream>, PeerId) {
    let id = select(server, client, index);
    let mut stream = attach(server, id);
    let tablet = Identity::load(IdentityRecord {
        certificate: record.certificate.clone(),
        private_key: record.private_key.clone(),
    })
    .unwrap();
    bounded(remote_hello(&mut stream, tablet.certificate())).await;
    let identified = event(server, id).await;
    server.link_event(id, identified).unwrap();
    if server.controller.snapshot().iter().any(|notice| matches!(notice, Notice::Link { connection, state: State::Pending, .. } if *connection == id)) {
        approve(server, client, id);
    }
    let host = server.identity.identity.clone();
    let (verified, tablet) =
        bounded(async { tokio::join!(event(server, id), remote_tls(stream, tablet, host)) }).await;
    server.link_event(id, verified).unwrap();
    server.authorize_next().unwrap();
    let completed = bounded(server.authority.next()).await.unwrap();
    server.authority_event(completed).unwrap();
    let pin = server.controller.lease(id).unwrap().ticket.peer.clone();
    (id, tablet, pin)
}
fn register(server: &mut Server, client: u64, request: u64, pin: PeerId) -> Result<()> {
    let (video, control) = super::super::registration::listeners();
    server.register(
        client,
        request,
        pin,
        Resource::DisplaySession,
        Credential::new([1; 32]),
        Profile::Display,
        vec![video.into(), control.into()],
    )
}
#[tokio::test]
async fn t921_duplicate_authenticated_certificates_do_not_route_media_arbitrarily() {
    let mut fixture = Fixture::configured(|settings| settings.session.channel_bytes = 4096).await;
    let (client, _remote) = fixture.client().await;
    let mut second = candidate();
    second.target = accessory::Target::new(1, &[3]).unwrap();
    second.address = 4;
    let mut third = candidate();
    third.target = accessory::Target::new(1, &[4]).unwrap();
    third.address = 5;
    let server = &mut fixture.server;
    server
        .inventory_result(Ok(vec![candidate(), second, third]))
        .unwrap();
    let record = IdentityRecord::generate().unwrap();
    let (first, tablet1, pin) = authenticate(server, client, 0, &record).await;
    let (second, tablet2, duplicate) = authenticate(server, client, 1, &record).await;
    assert_eq!(pin, duplicate);
    assert!(server.controller.lease(first).is_some());
    assert!(server.controller.lease(second).is_some());
    let result = register(server, client, 1, pin.clone());
    let rejected = result.is_err();
    let untouched = server.owner_id == 0 && server.routes.is_empty();
    // Finish real native owners even on the expected fail-before-fix path.
    if rejected {
        let (other, tablet3, other_pin) =
            authenticate(server, client, 2, &IdentityRecord::generate().unwrap()).await;
        register(server, client, 2, other_pin).unwrap();
        assert_eq!(server.routes[&1].connection, other);
        server.controller.failed(second).unwrap();
        server.close_requested().unwrap();
        register(server, client, 3, pin).unwrap();
        assert_eq!(server.routes[&2].connection, first);
        drop(tablet3);
    }
    drop((tablet1, tablet2));
    server.shutdown(Duration::from_secs(2)).await.unwrap();
    fixture.finish().await;
    assert!(
        rejected,
        "T921 ambiguous peer was sent to an arbitrary live TLS link"
    );
    assert!(untouched, "T921 ambiguity consumed registration ownership");
}
