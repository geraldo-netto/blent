use super::*;

fn step(rig: &mut Rig, now: i64) {
    if let Poll::Ready(result) = rig.poll(now) {
        result.unwrap();
    }
    rig.receive().unwrap();
    rig.send().unwrap();
}

fn block_final_fragment(rig: &mut Rig, commit: bool) {
    for _ in 0..500 {
        step(rig, 1);
        let selected = rig.host.active.as_ref().is_some_and(|active| {
            let expected = matches!(active.effect, Effect::Commit(_)) == commit;
            expected && active.remaining < 17
        });
        if selected {
            rig.io.lock().unwrap().flush_blocked = true;
            return;
        }
    }
    panic!("T872 final management fragment not selected");
}

fn pending_commit() -> (Rig, Lease) {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.message(Message::Prepared(lease.key));
    block_final_fragment(&mut rig, true);
    (rig, lease)
}

fn poll_and_receive(rig: &mut Rig) {
    for _ in 0..200 {
        if let Poll::Ready(result) = rig.poll(2) {
            result.unwrap();
        }
        rig.receive().unwrap();
    }
}

// Independent channel ordering is legal. Suppress unrelated initial media
// credits so these cases specifically exercise the management result barrier.
fn send_management_only(rig: &mut Rig) {
    for _ in 0..20 {
        let Some(frame) = rig.peer.next_frame().unwrap() else {
            return;
        };
        if frame.header.channel == Channel::Management {
            let mut bytes = [0; 40];
            frame.header.encode(&mut bytes).unwrap();
            let mut io = rig.io.lock().unwrap();
            io.input.extend(bytes);
            io.input.extend(frame.bytes);
        }
        rig.peer.finish(true).unwrap();
    }
    panic!("T872 bounded peer management fixture exhausted");
}

#[tokio::test]
async fn t872_early_media_credit_waits_for_commit_flush() {
    let (mut rig, lease) = pending_commit();
    rig.pump(2).unwrap();
    assert!(rig
        .messages
        .iter()
        .any(|message| { matches!(message, Message::Commit(key) if *key == lease.key) }));
    assert!(!rig.host.binding(&lease).unwrap().installed);
    assert!(!rig.host.current(&lease));
    assert!(rig.host.transport.is_some());
    assert!(rig.host.active.is_some());
    rig.io.lock().unwrap().flush_blocked = false;
    rig.pump(3).unwrap();
    assert!(rig.host.binding(&lease).unwrap().installed);
    rig.message(Message::Result(lease.key, Outcome::Admitted));
    rig.pump(4).unwrap();
    assert!(rig.host.current(&lease));
}

#[tokio::test]
async fn t872_early_admitted_waits_for_complete_commit_publication() {
    let (mut rig, lease) = pending_commit();
    poll_and_receive(&mut rig);
    rig.message(Message::Result(lease.key, Outcome::Admitted));
    send_management_only(&mut rig);
    poll_and_receive(&mut rig);
    assert!(matches!(
        rig.host.binding(&lease).unwrap().deferred,
        Some(Message::Result(_, Outcome::Admitted))
    ));
    assert!(!rig.host.current(&lease));
    rig.io.lock().unwrap().flush_blocked = false;
    rig.pump(3).unwrap();
    assert!(rig.host.current(&lease));
    assert!(rig.host.binding(&lease).unwrap().deferred.is_none());
}

#[tokio::test]
async fn t872_early_revoked_waits_for_complete_revoke_publication() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    block_final_fragment(&mut rig, false);
    poll_and_receive(&mut rig);
    rig.message(Message::Result(lease.key, Outcome::Revoked));
    send_management_only(&mut rig);
    poll_and_receive(&mut rig);
    assert!(matches!(
        rig.host.binding(&lease).unwrap().deferred,
        Some(Message::Result(_, Outcome::Revoked))
    ));
    assert!(!rig.host.binding(&lease).unwrap().acknowledged);
    rig.host.native_retired(&lease).unwrap();
    assert!(rig.host.slots[1].is_some());
    rig.io.lock().unwrap().flush_blocked = false;
    let events = rig.pump(3).unwrap();
    assert!(events.contains(&Event::State(lease, State::Retired)));
    assert!(rig.host.slots[1].is_none());
}

#[tokio::test]
async fn t872_permission_then_preparation_keeps_authority_ungranted() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.message(Message::Result(lease.key, Outcome::PermissionRequired));
    let events = rig.pump(1).unwrap();
    assert!(events.contains(&Event::State(lease.clone(), State::PermissionRequired)));
    assert!(!rig.host.current(&lease));
    rig.message(Message::Prepared(lease.key));
    rig.pump(2).unwrap();
    assert!(rig.host.binding(&lease).unwrap().installed);
    assert!(!rig.host.current(&lease));
    rig.message(Message::Result(lease.key, Outcome::Admitted));
    rig.pump(3).unwrap();
    assert!(rig.host.current(&lease));
}

fn acknowledge_uninstalled(rig: &mut Rig, lease: &Lease) {
    rig.message(Message::Result(lease.key, Outcome::Revoked));
    rig.pump(3).unwrap();
    assert!(!rig.host.binding(lease).unwrap().installed);
    rig.host.native_retired(lease).unwrap();
    assert!(rig.host.slots[lease.key.resource as usize - 1].is_none());
    let replacement = rig.register(lease.key.resource);
    assert_eq!(replacement.key.generation, lease.key.generation);
    assert!(replacement.key.request > lease.key.request);
}

#[tokio::test]
async fn t872_prepared_after_local_cancel_never_installs_channels() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    rig.message(Message::Prepared(lease.key));
    rig.pump(2).unwrap();
    assert!(!rig.messages.iter().any(|m| matches!(m, Message::Commit(_))));
    assert!(!rig.host.current(&lease));
    acknowledge_uninstalled(&mut rig, &lease);
}

#[tokio::test]
async fn t872_rejections_retire_without_consuming_mux_generation() {
    for rejection in [
        Outcome::Unavailable,
        Outcome::PermissionDenied,
        Outcome::Busy,
        Outcome::Failed,
    ] {
        let mut rig = Rig::new();
        let lease = rig.register(Resource::Camera);
        rig.pump(0).unwrap();
        rig.message(Message::Result(lease.key, rejection));
        let events = rig.pump(2).unwrap();
        assert!(events.contains(&Event::State(lease.clone(), State::Closing)));
        assert!(!rig.host.current(&lease));
        acknowledge_uninstalled(&mut rig, &lease);
    }
}

#[tokio::test]
async fn t872_fragmented_capabilities_and_malformed_management_fail_closed() {
    let mut rig = Rig::new();
    rig.pump(0).unwrap();
    rig.io.lock().unwrap().quantum = 1;
    rig.message(Message::Capabilities(15));
    rig.pump(1).unwrap();
    assert_eq!(rig.host.capabilities(), Some(15));
    rig.message(Message::Capabilities(15));
    assert!(rig.pump(2).is_err());
    assert!(rig.host.transport.is_none());
    for offset in [0, 4, 5, 6, 7] {
        let mut rig = Rig::new();
        rig.pump(0).unwrap();
        let mut bytes = [0; 284];
        let count = Message::Capabilities(15).encode(&mut bytes).unwrap();
        bytes[offset] = 255;
        rig.peer
            .offer(Channel::Management, bytes[..count].to_vec())
            .unwrap();
        assert!(rig.pump(1).is_err());
        assert!(rig.host.transport.is_none());
    }
}

#[tokio::test]
async fn t872_stale_management_keys_never_mutate_live_authority() {
    for request in 2..66 {
        let mut rig = Rig::new();
        let lease = rig.register(Resource::Camera);
        rig.pump(0).unwrap();
        let mut key = lease.key;
        key.request = request;
        rig.message(Message::Prepared(key));
        assert!(rig.pump(1).is_err());
        assert!(!rig.host.current(&lease));
        assert!(!rig.host.binding(&lease).unwrap().installed);
    }
}

fn invalid_limits(limits: Limits) {
    let io = Arc::new(Mutex::new(Io::default()));
    assert!(Session::new(Script(io), ticket(), limits).is_err());
}

#[tokio::test]
async fn t872_invalid_runtime_bounds_are_rejected_before_admission() {
    for channel_bytes in [0, 1, 4095, MAX_CREDIT + 1, u32::MAX] {
        invalid_limits(Limits {
            channel_bytes,
            ..Limits::default()
        });
    }
    for quantum in [0, MAX_PAYLOAD + 1, u32::MAX] {
        invalid_limits(Limits {
            quantum,
            ..Limits::default()
        });
    }
    for io_timeout in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        invalid_limits(Limits {
            io_timeout,
            ..Limits::default()
        });
    }
    for queued_messages in [0, 1025, usize::MAX] {
        invalid_limits(Limits {
            queued_messages,
            ..Limits::default()
        });
    }
}
