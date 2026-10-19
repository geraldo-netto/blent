use super::*;
use blent_config::connection::pairing::{Pairing, PeerId};
use std::os::unix::fs::PermissionsExt;

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
#[test]
fn t1102_identity_consumer_retains_original_store_after_provider_state_drop() {
    let root = root();
    let state = open(root.path(), 2).unwrap();
    let consumer = state.identity.clone();
    let certificate = consumer.certificate().to_vec();
    drop(state);
    let unexpected = open(root.path(), 2);
    let bypassed = unexpected.is_ok();
    drop(unexpected);
    assert!(
        !bypassed,
        "identity-store guard ended before its key consumer"
    );
    assert_eq!(consumer.certificate(), certificate);
    drop(consumer);
    assert!(open(root.path(), 2).is_ok());
}

#[test]
fn t853_persistent_identity_and_trust_survive_restart_with_exclusive_ownership() {
    let root = root();
    let mut state = open(root.path(), 2).unwrap();
    let certificate = state.identity.certificate().to_vec();
    assert!(open(root.path(), 2).is_err());
    assert!(state.trust.approved_peers().unwrap().is_empty());
    let peer = PeerId::parse("a".repeat(64)).unwrap();
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(peer.clone(), false, 0, 100).unwrap();
    pairing.approve(&ticket, true, 1).unwrap();
    state
        .trust
        .authenticated(&mut pairing, &ticket, &peer, 2)
        .unwrap();
    assert_eq!(state.trust.approved_peers().unwrap(), &[peer.clone()]);
    let owner = state.identity.clone();
    drop(state);
    assert!(open(root.path(), 2).is_err());
    drop(owner);
    let mut state = open(root.path(), 2).unwrap();
    assert_eq!(state.identity.certificate(), certificate);
    assert!(state.trust.trusted(&peer));
    state.trust.revoke(&mut Pairing::default(), &peer).unwrap();
    drop(state);
    assert!(open(root.path(), 2)
        .unwrap()
        .trust
        .approved_peers()
        .unwrap()
        .is_empty());
}
#[test]
fn t853_private_state_rejects_missing_unsafe_and_interrupted_storage() {
    let root = root();
    assert!(open(&root.path().join("missing"), 1).is_err());
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(open(root.path(), 1).is_err());
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(root.path().join("identity.pending"), []).unwrap();
    assert!(open(root.path(), 1).is_err());
    std::fs::remove_file(root.path().join("identity.pending")).unwrap();
    for capacity in [0, 1025, usize::MAX] {
        assert!(open(root.path(), capacity).is_err());
    }
    let state = open(root.path(), 1).unwrap();
    drop(state);
    std::fs::write(root.path().join("trust.pending"), []).unwrap();
    assert!(open(root.path(), 1).is_err());
}
