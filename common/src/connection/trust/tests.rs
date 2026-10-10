use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Disk {
    bytes: Option<Vec<u8>>,
    fail: bool,
    writes: usize,
}
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<Disk>>);
impl TrustStorage for Memory {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        let disk = self.0.borrow();
        ensure!(!disk.fail, "load failure");
        Ok(disk.bytes.clone())
    }
    fn replace(&mut self, bytes: &[u8]) -> Result<()> {
        let mut disk = self.0.borrow_mut();
        disk.writes += 1;
        ensure!(!disk.fail, "write failure");
        disk.bytes = Some(bytes.to_vec());
        Ok(())
    }
}
fn peer(value: char) -> PeerId {
    PeerId::parse(value.to_string().repeat(64)).unwrap()
}
fn approved(peer: PeerId, remember: bool) -> (Pairing, Lease) {
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(peer.clone(), false, 0, 1000).unwrap();
    pairing.approve(&ticket, remember, 1).unwrap();
    let lease = pairing.authenticated(&ticket, &peer, 2).unwrap();
    (pairing, lease)
}
#[test]
fn t853_approved_snapshot_requires_healthy_durable_store() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    assert!(trust.approved_peers().unwrap().is_empty());
    let (mut pairing, lease) = approved(peer('1'), true);
    trust.remember(&pairing, &lease).unwrap();
    assert_eq!(trust.approved_peers().unwrap(), &[peer('1')]);
    memory.0.borrow_mut().fail = true;
    assert!(trust.revoke(&mut pairing, &peer('1')).is_err());
    assert!(trust.approved_peers().is_err());
}
#[test]
fn t868_remember_restart_sorted_peers_and_revoke_without_implicit_authority() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), DEFAULT_CAPACITY).unwrap();
    assert!(trust.healthy());
    assert!(!trust.trusted(&peer('2')));
    let (mut pairing, lease) = approved(peer('2'), true);
    assert!(trust.remember(&pairing, &lease).unwrap());
    assert!(!trust.remember(&pairing, &lease).unwrap());
    let (other, next) = approved(peer('1'), true);
    trust.remember(&other, &next).unwrap();
    assert_eq!(memory.0.borrow().writes, 2);
    drop(trust);
    let mut trust = Trust::open(memory.clone(), peer('0'), DEFAULT_CAPACITY).unwrap();
    assert!(trust.trusted(&peer('1')) && trust.trusted(&peer('2')));
    let snapshot = codec::decode(memory.0.borrow().bytes.as_ref().unwrap()).unwrap();
    assert_eq!(snapshot.peers, vec![peer('1'), peer('2')]);
    assert_eq!(snapshot.revision, 2);
    assert!(trust.revoke(&mut pairing, &peer('2')).unwrap());
    assert!(!pairing.current(&lease));
    assert!(!trust.trusted(&peer('2')));
    assert!(!trust.revoke(&mut pairing, &peer('2')).unwrap());
    drop(trust);
    let trust = Trust::open(memory, peer('0'), DEFAULT_CAPACITY).unwrap();
    assert!(!trust.trusted(&peer('2')));
    assert!(trust.trusted(&peer('1')));
}
#[test]
fn t868_rejects_unproven_nonremembered_stale_and_changed_leases() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(peer('1'), false, 0, 100).unwrap();
    let forged = Lease {
        ticket: ticket.clone(),
        remember: true,
    };
    assert!(trust.remember(&pairing, &forged).is_err());
    pairing.approve(&ticket, true, 1).unwrap();
    assert!(trust.remember(&pairing, &forged).is_err());
    let lease = pairing.authenticated(&ticket, &peer('1'), 2).unwrap();
    trust.remember(&pairing, &lease).unwrap();
    pairing.cancel(&ticket);
    assert!(trust.remember(&pairing, &lease).is_err());
    let (other, lease) = approved(peer('2'), false);
    assert!(trust.remember(&other, &lease).is_err());
    assert_eq!(memory.0.borrow().writes, 1);
    assert!(trust.healthy());
}
#[test]
fn t868_capacity_has_no_eviction_and_invalid_load_is_preserved() {
    let memory = Memory::default();
    for capacity in [0, MAX_PEERS + 1, usize::MAX] {
        assert!(Trust::open(memory.clone(), peer('0'), capacity).is_err());
    }
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    let (first, lease) = approved(peer('1'), true);
    trust.remember(&first, &lease).unwrap();
    let (second, lease) = approved(peer('2'), true);
    assert!(trust.remember(&second, &lease).is_err());
    assert!(trust.trusted(&peer('1')));
    assert!(Trust::open(memory.clone(), peer('f'), 1).is_err());
    let saved = memory.0.borrow().bytes.clone();
    memory.0.borrow_mut().fail = true;
    assert!(Trust::open(memory.clone(), peer('0'), 1).is_err());
    assert_eq!(memory.0.borrow().bytes, saved);
}
#[test]
fn t868_failed_writes_and_counter_exhaustion_poison_reconnect_admission() {
    for exhausted in [false, true] {
        let memory = Memory::default();
        let snapshot = Snapshot {
            local: peer('0'),
            revision: i64::MAX as u64,
            peers: vec![peer('1'), peer('2')],
        };
        memory.0.borrow_mut().bytes = Some(codec::encode(&snapshot).unwrap());
        let mut trust = Trust::open(memory.clone(), peer('0'), MAX_PEERS).unwrap();
        if !exhausted {
            trust.snapshot.revision = 1;
            memory.0.borrow_mut().fail = true;
        }
        let (mut pairing, lease) = approved(peer('1'), true);
        assert!(trust.revoke(&mut pairing, &peer('1')).is_err());
        assert!(!pairing.current(&lease));
        assert!(!trust.healthy());
        assert!(!trust.trusted(&peer('2')));
        let (mut second, lease) = approved(peer('2'), true);
        assert!(trust.remember(&second, &lease).is_err());
        assert!(trust.revoke(&mut second, &peer('2')).is_err());
        assert!(!second.current(&lease));
    }
}
#[test]
fn t868_failed_remember_keeps_admission_closed_and_revoke_cancels_unsaved_peer() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    let (mut pairing, lease) = approved(peer('1'), true);
    assert!(!trust.revoke(&mut pairing, &peer('1')).unwrap());
    assert!(!pairing.current(&lease));
    let (pairing, lease) = approved(peer('1'), true);
    memory.0.borrow_mut().fail = true;
    assert!(trust.remember(&pairing, &lease).is_err());
    assert!(!trust.trusted(&peer('1')));
    assert!(!trust.healthy());
}
#[test]
fn t868_codec_golden_vectors_and_bounded_mutations() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../testdata/connection-trust-v1.json"
    ))
    .unwrap();
    for valid in data["valid"].as_array().unwrap() {
        check_valid(valid);
    }
    for invalid in data["invalid"].as_array().unwrap() {
        assert!(
            codec::decode(&unhex(invalid["hex"].as_str().unwrap())).is_err(),
            "{}",
            invalid["name"]
        );
    }
    let snapshot = Snapshot {
        local: peer('0'),
        revision: 1,
        peers: vec![peer('1')],
    };
    let bytes = codec::encode(&snapshot).unwrap();
    for seed in 0u32..4096 {
        let mut bad = bytes.clone();
        bad[80..84].copy_from_slice(&seed.wrapping_mul(0x9e3779b9).to_be_bytes());
        assert!(codec::decode(&bad).is_err());
    }
    for end in 0..bytes.len() {
        assert!(codec::decode(&bytes[..end]).is_err());
    }
}
fn check_valid(value: &serde_json::Value) {
    let bytes = unhex(value["hex"].as_str().unwrap());
    let snapshot = codec::decode(&bytes).unwrap();
    assert_eq!(snapshot.local.as_str(), value["local"].as_str().unwrap());
    assert_eq!(
        snapshot.revision.to_string(),
        value["revision"].as_str().unwrap()
    );
    let peers: Vec<_> = value["peers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| PeerId::parse(p.as_str().unwrap().into()).unwrap())
        .collect();
    assert_eq!(snapshot.peers, peers);
    assert_eq!(codec::encode(&snapshot).unwrap(), bytes);
}
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn t868_codec_encode_rejects_noncanonical_snapshots_and_capacity_lowering() {
    let memory = Memory::default();
    let mut snapshot = Snapshot {
        local: peer('0'),
        revision: 1,
        peers: vec![peer('1'), peer('2')],
    };
    memory.0.borrow_mut().bytes = Some(codec::encode(&snapshot).unwrap());
    assert!(Trust::open(memory, peer('0'), 1).is_err());
    snapshot.peers.reverse();
    assert!(codec::encode(&snapshot).is_err());
    snapshot.peers = vec![peer('1'); MAX_PEERS + 1];
    assert!(codec::encode(&snapshot).is_err());
    snapshot.peers.clear();
    for revision in [0, i64::MAX as u64 + 1, u64::MAX] {
        snapshot.revision = revision;
        assert!(codec::encode(&snapshot).is_err());
    }
}

#[test]
fn t868_codec_accepts_exact_maximum_and_rejects_oversized_record() {
    let peers = (0..MAX_PEERS)
        .map(|value| PeerId::parse(format!("{value:064x}")).unwrap())
        .collect();
    let snapshot = Snapshot {
        local: peer('f'),
        revision: 1,
        peers,
    };
    let bytes = codec::encode(&snapshot).unwrap();
    assert_eq!(bytes.len(), codec::MAX_RECORD);
    assert_eq!(codec::decode(&bytes).unwrap(), snapshot);
    let mut too_large = bytes;
    too_large.push(0);
    assert!(codec::decode(&too_large).is_err());
}

#[test]
fn t852_composition_requires_fresh_proof_after_restart_and_revoke() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    let mut pairing = Pairing::default();
    let ticket = trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
    assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Pending);
    pairing.approve(&ticket, true, 1).unwrap();
    let lease = trust
        .authenticated(&mut pairing, &ticket, &peer('1'), 2)
        .unwrap();
    assert!(pairing.current(&lease));
    drop(trust);
    pairing.cancel(&ticket);
    pairing.retired(&ticket).unwrap();
    let mut trust = Trust::open(memory, peer('0'), 1).unwrap();
    let ticket = trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
    assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Approved);
    let lease = trust
        .authenticated(&mut pairing, &ticket, &peer('1'), 2)
        .unwrap();
    assert!(!lease.remember);
    assert!(pairing.current(&lease));
    trust.revoke(&mut pairing, &peer('1')).unwrap();
    assert!(trust.begin(&mut pairing, peer('1'), 0, 100).is_err());
    pairing.retired(&ticket).unwrap();
    trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
    assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Pending);
}
#[test]
fn t852_failed_persistence_or_proof_cancels_matching_attempt_and_keeps_retirement_barrier() {
    for fail_storage in [false, true] {
        let memory = Memory::default();
        let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
        let mut pairing = Pairing::default();
        let ticket = trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
        pairing.approve(&ticket, true, 1).unwrap();
        memory.0.borrow_mut().fail = fail_storage;
        let proof = if fail_storage { peer('1') } else { peer('2') };
        assert!(trust
            .authenticated(&mut pairing, &ticket, &proof, 2)
            .is_err());
        assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Closing);
        assert!(trust.begin(&mut pairing, peer('1'), 3, 100).is_err());
        pairing.retired(&ticket).unwrap();
        if fail_storage {
            assert!(trust.begin(&mut pairing, peer('1'), 3, 100).is_err());
        }
        assert!(memory.0.borrow().bytes.is_none());
    }
}
#[test]
fn t852_stale_completion_cannot_cancel_another_attempt_and_health_is_rechecked() {
    let memory = Memory::default();
    let mut trust = Trust::open(memory.clone(), peer('0'), 1).unwrap();
    let mut pairing = Pairing::default();
    let stale = trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
    pairing.cancel(&stale);
    pairing.retired(&stale).unwrap();
    let ticket = trust.begin(&mut pairing, peer('2'), 0, 100).unwrap();
    pairing.approve(&ticket, true, 1).unwrap();
    assert!(trust
        .authenticated(&mut pairing, &stale, &peer('1'), 2)
        .is_err());
    assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Approved);
    let (mut other, lease) = approved(peer('3'), true);
    memory.0.borrow_mut().fail = true;
    assert!(trust.remember(&other, &lease).is_err());
    assert!(trust
        .authenticated(&mut pairing, &ticket, &peer('2'), 2)
        .is_err());
    assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Closing);
    // All other live connections are invalidated by the shared broker on failure.
    assert!(trust.revoke(&mut other, &peer('3')).is_err());
    assert!(!other.current(&lease));
}

#[test]
fn t852_bounded_invalid_completion_cannot_retarget_live_attempt() {
    for offset in 1..=256 {
        let mut trust = Trust::open(Memory::default(), peer('0'), 1).unwrap();
        let mut pairing = Pairing::default();
        let ticket = trust.begin(&mut pairing, peer('1'), 0, 100).unwrap();
        pairing.approve(&ticket, true, 1).unwrap();
        let stale = Ticket {
            epoch: ticket.epoch + offset,
            peer: ticket.peer.clone(),
        };
        assert!(trust
            .authenticated(&mut pairing, &stale, &peer('1'), 2)
            .is_err());
        assert_eq!(pairing.phase(), crate::connection::pairing::Phase::Approved);
        let lease = trust
            .authenticated(&mut pairing, &ticket, &peer('1'), 2)
            .unwrap();
        assert!(pairing.current(&lease));
    }
}
