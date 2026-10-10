//! T852 integration: actual private persistence, stock TLS and pairing policy.
use super::*;
use blent_config::connection::{
    pairing::{Pairing, PeerId, Phase},
    trust::{Trust, DEFAULT_CAPACITY},
};
use identity_store::{linux::LinuxIdentityStore, load_or_create};
use std::os::unix::fs::PermissionsExt;
use trust_store::LinuxTrustStore;

struct Endpoint {
    _identity_owner: LinuxIdentityStore,
    identity: Identity,
    trust: Trust<LinuxTrustStore>,
    pairing: Pairing,
}
fn pin(peer: &PeerCertificate) -> PeerId {
    PeerId::parse(
        peer.fingerprint()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
    .unwrap()
}
fn open(root: &std::path::Path) -> Endpoint {
    let mut owner = LinuxIdentityStore::open(root).unwrap();
    let identity = load_or_create(&mut owner, IdentityRecord::generate).unwrap();
    let local = pin(&PeerCertificate::parse(identity.certificate().to_vec()).unwrap());
    let trust = Trust::open(
        LinuxTrustStore::open(root).unwrap(),
        local,
        DEFAULT_CAPACITY,
    )
    .unwrap();
    Endpoint {
        _identity_owner: owner,
        identity,
        trust,
        pairing: Pairing::default(),
    }
}
fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
async fn exchange(host: &mut Endpoint, tablet: &mut Endpoint, expected: Phase) {
    let host_peer = PeerCertificate::parse(host.identity.certificate().to_vec()).unwrap();
    let tablet_peer = PeerCertificate::parse(tablet.identity.certificate().to_vec()).unwrap();
    let h = host
        .trust
        .begin(&mut host.pairing, pin(&tablet_peer), 0, 1000)
        .unwrap();
    let t = tablet
        .trust
        .begin(&mut tablet.pairing, pin(&host_peer), 0, 1000)
        .unwrap();
    assert_eq!(host.pairing.phase(), expected);
    assert_eq!(tablet.pairing.phase(), expected);
    if expected == Phase::Pending {
        host.pairing.approve(&h, true, 1).unwrap();
        tablet.pairing.approve(&t, true, 1).unwrap();
    }
    let (left, right) = tokio::io::duplex(8192);
    let (host_io, tablet_io) = tokio::join!(
        accept(left, &host.identity, &tablet_peer),
        connect(right, &tablet.identity, &host_peer)
    );
    let mut host_io = host_io.unwrap();
    let mut tablet_io = tablet_io.unwrap();
    let host_lease = host
        .trust
        .authenticated(&mut host.pairing, &h, &pin(&tablet_peer), 2)
        .unwrap();
    let tablet_lease = tablet
        .trust
        .authenticated(&mut tablet.pairing, &t, &pin(&host_peer), 2)
        .unwrap();
    assert!(host.pairing.current(&host_lease) && tablet.pairing.current(&tablet_lease));
    host_io.write_all(b"persisted identity").await.unwrap();
    host_io.flush().await.unwrap();
    let mut received = [0; 18];
    tablet_io.read_exact(&mut received).await.unwrap();
    assert_eq!(&received, b"persisted identity");
}
#[tokio::test]
async fn t852_persisted_identities_and_pins_survive_restart_require_tls_and_revoke() {
    let hroot = root();
    let troot = root();
    let mut host = open(hroot.path());
    let mut tablet = open(troot.path());
    let original = host.identity.certificate().to_vec();
    exchange(&mut host, &mut tablet, Phase::Pending).await;
    drop(host);
    drop(tablet);
    let mut host = open(hroot.path());
    let mut tablet = open(troot.path());
    assert_eq!(host.identity.certificate(), original);
    exchange(&mut host, &mut tablet, Phase::Approved).await;
    let hp = pin(&PeerCertificate::parse(host.identity.certificate().to_vec()).unwrap());
    let tp = pin(&PeerCertificate::parse(tablet.identity.certificate().to_vec()).unwrap());
    host.trust.revoke(&mut host.pairing, &tp).unwrap();
    tablet.trust.revoke(&mut tablet.pairing, &hp).unwrap();
    assert_eq!(host.pairing.phase(), Phase::Closing);
    drop(host);
    drop(tablet);
    let mut host = open(hroot.path());
    let mut tablet = open(troot.path());
    exchange(&mut host, &mut tablet, Phase::Pending).await;
}
#[tokio::test]
async fn t852_failed_stock_tls_does_not_persist_a_remembered_approval() {
    let hroot = root();
    let troot = root();
    let mut host = open(hroot.path());
    let tablet = open(troot.path());
    let stranger = Identity::load(IdentityRecord::generate().unwrap()).unwrap();
    let host_peer = PeerCertificate::parse(host.identity.certificate().to_vec()).unwrap();
    let wrong_peer = PeerCertificate::parse(stranger.certificate().to_vec()).unwrap();
    let ticket = host
        .trust
        .begin(&mut host.pairing, pin(&wrong_peer), 0, 100)
        .unwrap();
    host.pairing.approve(&ticket, true, 1).unwrap();
    let (left, right) = tokio::io::duplex(8192);
    let (result, _peer) = tokio::join!(
        accept(left, &host.identity, &wrong_peer),
        connect(right, &tablet.identity, &host_peer)
    );
    assert!(result.is_err());
    assert!(!hroot.path().join("trust.bin").exists());
    assert!(!host.trust.trusted(&pin(&wrong_peer)));
    host.pairing.cancel(&ticket);
    assert!(host
        .trust
        .begin(&mut host.pairing, pin(&wrong_peer), 1, 100)
        .is_err());
    host.pairing.retired(&ticket).unwrap();
}
