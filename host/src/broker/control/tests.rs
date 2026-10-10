use super::*;
use blent_config::{
    connection::trust::{codec, Snapshot, Trust, TrustStorage},
    retirement::{Admission as NativeAdmission, Pool},
};
use std::{cell::Cell, rc::Rc};
type Control = Controller<u64, u64>;
struct Store {
    bytes: Option<Vec<u8>>,
    fail: Rc<Cell<bool>>,
    writes: Rc<Cell<usize>>,
}
impl TrustStorage for Store {
    fn load(&mut self) -> Result<Option<Vec<u8>>> {
        Ok(self.bytes.clone())
    }
    fn replace(&mut self, bytes: &[u8]) -> Result<()> {
        self.writes.set(self.writes.get() + 1);
        ensure!(!self.fail.get(), "injected storage failure");
        self.bytes = Some(bytes.to_vec());
        Ok(())
    }
}
struct Rig {
    control: Control,
    client: u64,
    trust: Trust<Store>,
    pool: Pool,
    fail: Rc<Cell<bool>>,
    writes: Rc<Cell<usize>>,
}
fn peer(n: u64) -> PeerId {
    PeerId::parse(format!("{n:064x}")).unwrap()
}
fn device(key: u64) -> Device<u64, u64> {
    Device {
        key,
        payload: key,
        vendor: 0x18d1,
        product: 0x2d00,
        accessory: true,
    }
}
impl Rig {
    fn new(saved: Vec<PeerId>) -> Self {
        let fail = Rc::new(Cell::new(false));
        let writes = Rc::new(Cell::new(0));
        let bytes = codec::encode(&Snapshot {
            local: peer(0),
            revision: 1,
            peers: saved.clone(),
        })
        .unwrap();
        let store = Store {
            bytes: Some(bytes),
            fail: fail.clone(),
            writes: writes.clone(),
        };
        let trust = Trust::open(store, peer(0), 64).unwrap();
        let mut control = Control::new(peer(0), saved, Limits::default()).unwrap();
        let client = control.admit_client().unwrap();
        control
            .replace_devices(vec![device(1), device(2), device(3), device(4)])
            .unwrap();
        Self {
            control,
            client,
            trust,
            pool: Pool::new(64).unwrap(),
            fail,
            writes,
        }
    }
    fn start(&mut self, key: u64) -> u64 {
        let target = *self
            .control
            .devices
            .iter()
            .find(|(_, device)| device.key == key)
            .unwrap()
            .0;
        match self
            .control
            .command(
                self.client,
                Command::Connect {
                    revision: self.control.revision,
                    target,
                },
                0,
            )
            .unwrap()
        {
            Action::Start { connection, target } => {
                assert_eq!(target, key);
                connection
            }
            _ => panic!("expected native start"),
        }
    }
    fn pending(&mut self, key: u64, remote: PeerId) -> (u64, Ticket, NativeAdmission) {
        let id = self.start(key);
        let hold = self.pool.reserve().unwrap();
        self.control.attach(id, hold.completion()).unwrap();
        self.control.identified(id, remote, 1).unwrap();
        let ticket = self.control.links[&id].ticket.clone().unwrap();
        (id, ticket, hold)
    }
    fn approve(&mut self, id: u64, ticket: &Ticket, remember: bool) {
        assert!(matches!(
            self.control
                .command(
                    self.client,
                    Command::Decide {
                        connection: id,
                        ticket: ticket.clone(),
                        allow: true,
                        remember
                    },
                    2
                )
                .unwrap(),
            Action::Authenticate { .. }
        ));
    }
    fn job(&mut self, id: u64, ticket: &Ticket) -> AuthorityJob {
        self.control
            .prepare_authentication(id, ticket, ticket.peer.clone(), 3)
            .unwrap()
    }
    fn disconnect(&mut self, id: u64, ticket: Option<Ticket>) {
        assert!(matches!(
            self.control
                .command(
                    self.client,
                    Command::Disconnect {
                        connection: id,
                        ticket
                    },
                    5
                )
                .unwrap(),
            Action::Retire(_)
        ));
    }
    fn revoke(&mut self, remote: PeerId) {
        assert!(matches!(
            self.control
                .command(self.client, Command::Revoke { peer: remote }, 5)
                .unwrap(),
            Action::Retire(_)
        ));
    }
}

#[test]
fn t853_snapshot_ids_and_physical_tombstones_are_exact() {
    let mut rig = Rig::new(vec![peer(8)]);
    let before = rig.control.snapshot();
    assert_eq!(before.len(), 7);
    for notice in &before {
        Notice::decode(&notice.encode().unwrap()).unwrap();
    }
    assert!(matches!(
        rig.control
            .command(rig.client, Command::Inspect, 0)
            .unwrap(),
        Action::Snapshot(_)
    ));
    let old_revision = rig.control.revision;
    let old_target = *rig.control.devices.keys().next().unwrap();
    let first = rig.start(1);
    rig.control.replace_devices(vec![device(1)]).unwrap();
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Connect {
                revision: old_revision,
                target: old_target
            },
            0
        )
        .is_err());
    let target = *rig.control.devices.keys().next().unwrap();
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Connect {
                revision: rig.control.revision,
                target
            },
            0
        )
        .is_err());
    rig.control.start_failed(first).unwrap();
    assert_eq!(rig.control.collect_retired().unwrap(), vec![first]);
    let next = rig.start(1);
    assert_ne!(next, first);
    assert!(rig.control.drop_client(rig.client));
    assert!(!rig.control.drop_client(rig.client));
    assert!(rig
        .control
        .command(rig.client, Command::Inspect, 0)
        .is_err());
    assert!(rig.control.links.contains_key(&next));
}

#[test]
fn t853_approval_proof_persistence_and_reconnect_remain_distinct() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, hold) = rig.pending(1, peer(1));
    assert!(rig
        .control
        .prepare_authentication(id, &ticket, peer(1), 2)
        .is_err());
    rig.approve(id, &ticket, true);
    assert!(rig
        .control
        .prepare_authentication(id, &ticket, peer(2), 3)
        .is_err());
    let job = rig.job(id, &ticket);
    assert!(rig.control.pairing(id).is_err());
    assert!(rig.control.lease(id).is_none());
    assert!(rig.control.next_authority().is_err());
    let reply = job.run(&mut rig.trust);
    let admitted = rig.control.complete_authority(reply, 4).unwrap().unwrap();
    assert_eq!(admitted.connection, id);
    assert!(rig.control.pairing(id).unwrap().current(&admitted.lease));
    assert_eq!(rig.control.lease(id), Some(&admitted.lease));
    assert_eq!(rig.writes.get(), 1);
    assert!(rig
        .control
        .snapshot()
        .iter()
        .any(|notice| matches!(notice, Notice::Saved { peer: saved, .. } if saved == &peer(1))));
    rig.disconnect(id, Some(ticket));
    rig.control.native_finished(id).unwrap();
    assert!(rig.control.collect_retired().unwrap().is_empty());
    drop(hold);
    assert_eq!(rig.control.collect_retired().unwrap(), vec![id]);
    let (next, next_ticket, _hold) = rig.pending(1, peer(1));
    assert_eq!(rig.control.links[&next].state, State::Approved);
    let reply = rig.job(next, &next_ticket).run(&mut rig.trust);
    assert!(
        !rig.control
            .complete_authority(reply, 4)
            .unwrap()
            .unwrap()
            .lease
            .remember
    );
    assert_eq!(rig.writes.get(), 1);
}

#[test]
fn t853_revoke_during_remember_cancels_before_durable_delete() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, hold) = rig.pending(1, peer(1));
    let (other, other_ticket, _other_hold) = rig.pending(2, peer(2));
    rig.approve(id, &ticket, true);
    let job = rig.job(id, &ticket);
    rig.approve(other, &other_ticket, false);
    assert!(rig
        .control
        .prepare_authentication(other, &other_ticket, peer(2), 4)
        .is_err());
    rig.revoke(peer(1));
    rig.control.native_finished(id).unwrap();
    drop(hold);
    assert!(rig.control.collect_retired().unwrap().is_empty());
    let reply = job.run(&mut rig.trust);
    assert!(rig.trust.trusted(&peer(1)));
    assert!(rig.control.complete_authority(reply, 6).unwrap().is_none());
    assert_eq!(rig.control.saved, vec![peer(1)]);
    assert!(rig.control.lease(id).is_none());
    let job = rig.control.next_authority().unwrap().unwrap();
    assert!(rig.control.collect_retired().unwrap().is_empty());
    assert!(rig
        .control
        .complete_authority(job.run(&mut rig.trust), 7)
        .unwrap()
        .is_none());
    assert!(!rig.trust.trusted(&peer(1)));
    assert_eq!(rig.writes.get(), 2);
    assert_eq!(rig.control.collect_retired().unwrap(), vec![id]);
    assert!(rig.control.next_authority().unwrap().is_none());
    let reply = rig.job(other, &other_ticket).run(&mut rig.trust);
    assert!(rig.control.complete_authority(reply, 8).unwrap().is_some());
}

#[test]
fn t853_disconnect_and_expiry_reject_delayed_success_without_faking_retirement() {
    for expires in [false, true] {
        let mut rig = Rig::new(vec![]);
        let (id, ticket, hold) = rig.pending(1, peer(1));
        rig.approve(id, &ticket, true);
        let job = rig.job(id, &ticket);
        let now = if expires { 120_001 } else { 4 };
        if expires {
            assert_eq!(rig.control.tick(now).unwrap(), vec![id]);
        } else {
            rig.disconnect(id, Some(ticket.clone()));
        }
        rig.control.quarantine(id).unwrap();
        assert!(rig
            .control
            .complete_authority(job.run(&mut rig.trust), now)
            .unwrap()
            .is_none());
        assert!(rig.trust.trusted(&peer(1)));
        assert_eq!(
            rig.control.pairing(id).unwrap().phase(),
            blent_config::connection::pairing::Phase::Closing
        );
        drop(hold);
        assert!(rig.control.collect_retired().unwrap().is_empty());
        rig.control.native_finished(id).unwrap();
        assert_eq!(rig.control.collect_retired().unwrap(), vec![id]);
    }
}

#[test]
fn t853_storage_failure_poison_cancels_all_links_and_keeps_native_ownership() {
    let mut rig = Rig::new(vec![]);
    let (first, ticket, _first_hold) = rig.pending(1, peer(1));
    let (second, other_ticket, _second_hold) = rig.pending(2, peer(2));
    rig.approve(first, &ticket, true);
    let job = rig.job(first, &ticket);
    rig.approve(second, &other_ticket, false);
    assert!(rig
        .control
        .prepare_authentication(second, &other_ticket, peer(2), 3)
        .is_err());
    rig.fail.set(true);
    assert!(rig
        .control
        .complete_authority(job.run(&mut rig.trust), 4)
        .is_err());
    assert!(!rig.control.healthy);
    assert!(rig
        .control
        .links
        .values()
        .all(|link| link.state == State::Closing));
    assert_eq!(rig.control.closing(), vec![first, second]);
    assert!(rig.control.collect_retired().unwrap().is_empty());
    assert!(rig
        .control
        .prepare_authentication(second, &other_ticket, peer(2), 4)
        .is_err());
}

#[test]
fn t853_denial_stale_callbacks_and_native_start_failure_are_bounded() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, hold) = rig.pending(1, peer(1));
    let stale = Ticket {
        epoch: ticket.epoch + 1,
        peer: peer(1),
    };
    for candidate in [
        stale,
        Ticket {
            epoch: ticket.epoch,
            peer: peer(2),
        },
    ] {
        assert!(rig
            .control
            .command(
                rig.client,
                Command::Decide {
                    connection: id,
                    ticket: candidate.clone(),
                    allow: true,
                    remember: false
                },
                2
            )
            .is_err());
        assert!(rig
            .control
            .command(
                rig.client,
                Command::Disconnect {
                    connection: id,
                    ticket: Some(candidate)
                },
                2
            )
            .is_err());
    }
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Decide {
                connection: id,
                ticket: ticket.clone(),
                allow: false,
                remember: false
            },
            2
        )
        .is_ok());
    assert!(rig
        .control
        .command(
            rig.client,
            Command::Decide {
                connection: id,
                ticket: ticket.clone(),
                allow: true,
                remember: false
            },
            2
        )
        .is_err());
    assert!(rig.control.identified(id, peer(1), 3).is_err());
    assert!(rig.control.start_failed(id).is_err());
    assert!(rig.control.attach(id, hold.completion()).is_err());
    rig.control.failed(id).unwrap();
    rig.control.native_finished(id).unwrap();
    drop(hold);
    assert_eq!(rig.control.collect_retired().unwrap(), vec![id]);
    assert!(rig.control.failed(id).is_err());
    let next = rig.start(1);
    assert!(rig.control.native_finished(next).is_err());
    assert!(rig.control.quarantine(next).is_err());
    assert!(rig.control.identified(next, peer(1), 1).is_err());
    rig.disconnect(next, None);
    rig.control.start_failed(next).unwrap();
    assert!(rig.control.start_failed(next).is_err());
    assert_eq!(rig.control.collect_retired().unwrap(), vec![next]);
}

#[test]
fn t853_revoke_without_live_pairing_and_failed_delete_stay_fail_closed() {
    let mut rig = Rig::new(vec![peer(9)]);
    rig.revoke(peer(9));
    rig.revoke(peer(9));
    assert_eq!(rig.control.revokes.len(), 1);
    let reply = rig
        .control
        .next_authority()
        .unwrap()
        .unwrap()
        .run(&mut rig.trust);
    rig.control.complete_authority(reply, 0).unwrap();
    assert!(!rig.trust.trusted(&peer(9)));
    assert_eq!(rig.writes.get(), 1);
    let (id, ticket, _hold) = rig.pending(1, peer(8));
    rig.approve(id, &ticket, true);
    let reply = rig.job(id, &ticket).run(&mut rig.trust);
    rig.control.complete_authority(reply, 4).unwrap();
    rig.revoke(peer(8));
    rig.fail.set(true);
    let reply = rig
        .control
        .next_authority()
        .unwrap()
        .unwrap()
        .run(&mut rig.trust);
    assert!(rig.control.complete_authority(reply, 5).is_err());
    assert!(rig.control.lease(id).is_none());
    assert_eq!(rig.control.saved, vec![peer(8)]);
}

mod bounds;

#[test]
fn t853_saved_notice_remains_until_durable_revoke_completes() {
    let mut rig = Rig::new(vec![peer(9)]);
    rig.revoke(peer(9));
    assert!(
        rig.control
            .snapshot()
            .iter()
            .any(|notice| matches!(notice, Notice::Saved { peer: saved, .. } if saved == &peer(9))),
        "queued revoke is not durable deletion"
    );
    let job = rig.control.next_authority().unwrap().unwrap();
    assert_eq!(rig.control.saved, vec![peer(9)]);
    let reply = job.run(&mut rig.trust);
    rig.control.complete_authority(reply, 6).unwrap();
    assert!(rig.control.saved.is_empty());
}

#[test]
fn t853_authority_metadata_correlates_exact_job_and_durable_reply() {
    let mut rig = Rig::new(vec![]);
    let (id, ticket, _hold) = rig.pending(1, peer(1));
    rig.approve(id, &ticket, true);
    let job = rig.job(id, &ticket);
    let authenticated = job.operation().clone();
    assert_eq!(authenticated.kind, AuthorityKind::Authenticate);
    assert_eq!(authenticated.connection, Some(id));
    assert_eq!(authenticated.peer, peer(1));
    assert!(authenticated.id > id);
    let reply = job.run(&mut rig.trust);
    assert_eq!(reply.operation(), &authenticated);
    rig.control.complete_authority(reply, 4).unwrap();
    rig.revoke(peer(1));
    let job = rig.control.next_authority().unwrap().unwrap();
    let revoked = job.operation().clone();
    assert_eq!(revoked.kind, AuthorityKind::Revoke);
    assert_eq!(revoked.connection, Some(id));
    assert_eq!(revoked.peer, peer(1));
    assert!(revoked.id > authenticated.id);
    let reply = job.run(&mut rig.trust);
    assert_eq!(reply.operation(), &revoked);
    rig.control.complete_authority(reply, 5).unwrap();
    rig.revoke(peer(9));
    let job = rig.control.next_authority().unwrap().unwrap();
    assert_eq!(job.operation().connection, None);
    assert_eq!(job.operation().peer, peer(9));
    assert!(job.operation().id > revoked.id);
    rig.control
        .complete_authority(job.run(&mut rig.trust), 6)
        .unwrap();
}
