use super::*;
#[tokio::test]
async fn t872_stale_equal_length_completion_cannot_release_new_borrow() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.peer.offer(Channel::Camera, vec![1; 32]).unwrap();
    rig.pump(3).unwrap();
    let mut bytes = [0; 16];
    let first = rig
        .host
        .read(&lease, Channel::Camera, &mut bytes)
        .unwrap()
        .unwrap();
    rig.host.consumed(&lease, Channel::Camera, first).unwrap();
    let second = rig
        .host
        .read(&lease, Channel::Camera, &mut bytes)
        .unwrap()
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.count, second.count);
    assert!(
        rig.host.consumed(&lease, Channel::Camera, first).is_err(),
        "T872 stale completion released new equal-length borrow"
    );
    rig.host.consumed(&lease, Channel::Camera, second).unwrap();
}
#[tokio::test]
async fn t872_consumer_completion_reannounces_buffered_remainder() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.peer.offer(Channel::Camera, vec![1; 32]).unwrap();
    rig.pump(3).unwrap();
    let mut bytes = [0; 8];
    let read = rig
        .host
        .read(&lease, Channel::Camera, &mut bytes)
        .unwrap()
        .unwrap();
    rig.host.consumed(&lease, Channel::Camera, read).unwrap();
    assert!(
        matches!(
            rig.poll(3),
            Poll::Ready(Ok(Event::Readable(_, Channel::Camera)))
        ),
        "T872 buffered remainder lost readiness after consumer completion"
    );
}
#[tokio::test]
async fn t872_readiness_memory_is_bounded_when_consumer_does_not_poll_events() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.peer.offer(Channel::Camera, vec![1; 128]).unwrap();
    rig.pump(3).unwrap();
    for _ in 0..127 {
        let read = rig
            .host
            .read(&lease, Channel::Camera, &mut [0])
            .unwrap()
            .unwrap();
        rig.host.consumed(&lease, Channel::Camera, read).unwrap();
    }
    assert!(
        rig.host.events.len() + rig.host.readable.iter().flatten().count() <= 5,
        "T872 consumer accumulated duplicate readiness events: {}",
        rig.host.events.len()
    );
}
#[tokio::test]
async fn t872_idle_tick_and_stale_readiness_preserve_retirement() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Microphone);
    rig.pump(0).unwrap();
    assert!(rig.host.tick(-1).is_err());
    rig.host.tick(1000).unwrap();
    assert!(rig.host.binding(&lease).unwrap().closing);
    rig.pump(1001).unwrap();
    rig.message(Message::Result(lease.key, Outcome::Revoked));
    rig.pump(1002).unwrap();
    rig.host.native_retired(&lease).unwrap();
    rig.host.readable[Channel::Microphone as usize - 1] = Some(lease.key);
    assert!(matches!(
        rig.host.next_notice(),
        Some(Event::State(_, State::Retired))
    ));
    assert!(rig.host.next_notice().is_none());
}
#[tokio::test]
async fn t872_audio_channels_and_peer_revoke_remain_independent() {
    let mut rig = Rig::new();
    let microphone = rig.admit(Resource::Microphone);
    let speaker = rig.admit(Resource::Speakers);
    rig.peer
        .offer(Channel::Microphone, b"microphone".to_vec())
        .unwrap();
    rig.peer
        .offer(Channel::Speaker, b"speaker".to_vec())
        .unwrap();
    rig.pump(3).unwrap();
    for (lease, channel, count) in [
        (&microphone, Channel::Microphone, 10),
        (&speaker, Channel::Speaker, 7),
    ] {
        let read = rig
            .host
            .read(lease, channel, &mut [0; 16])
            .unwrap()
            .unwrap();
        assert_eq!(read.count, count);
        rig.host.consumed(lease, channel, read).unwrap();
    }
    rig.message(Message::Revoke(microphone.key, Reason::PermissionRevoked));
    rig.pump(4).unwrap();
    assert!(!rig.host.current(&microphone));
    assert!(rig.host.current(&speaker));
    rig.message(Message::Commit(speaker.key));
    assert!(rig.pump(5).is_err());
}
#[tokio::test]
async fn t872_tiny_consumption_does_not_starve_peer_revocation() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.peer.offer(Channel::Camera, vec![1; 4096]).unwrap();
    rig.pump(3).unwrap();
    let read = rig
        .host
        .read(&lease, Channel::Camera, &mut [0])
        .unwrap()
        .unwrap();
    rig.host.consumed(&lease, Channel::Camera, read).unwrap();
    rig.message(Message::Revoke(lease.key, Reason::Requested));
    rig.send().unwrap();
    for _ in 0..128 {
        if !rig.host.current(&lease) {
            break;
        }
        if let Poll::Ready(result) = rig.poll(4) {
            if matches!(result.unwrap(), Event::Readable(_, Channel::Camera)) {
                let read = rig
                    .host
                    .read(&lease, Channel::Camera, &mut [0])
                    .unwrap()
                    .unwrap();
                rig.host.consumed(&lease, Channel::Camera, read).unwrap();
            }
        }
    }
    assert!(
        !rig.host.current(&lease),
        "T872 repeated readiness starved peer revocation"
    );
}
#[tokio::test]
async fn t872_local_pairing_retries_do_not_change_peer_wire_namespace() {
    let mut rig = Rig::new();
    let mut local = ticket();
    local.epoch = 37;
    rig.host = Session::new(
        Script(rig.io.clone()),
        local,
        Limits {
            channel_bytes: 4096,
            quantum: 17,
            ..Limits::default()
        },
    )
    .unwrap();
    let lease = rig.admit(Resource::DisplaySession);
    assert_eq!(lease.connection.epoch, 37);
    assert!(rig.host.current(&lease));
}
