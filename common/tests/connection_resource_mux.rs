//! T851: compose policy with real mux credits/terminals, not mock retirement.
use blent_config::connection::{
    management::Resource,
    mux::Multiplexer,
    pairing::{PeerId, Ticket},
    resources::{Install, Resources, Role},
    wire::{Channel, Kind},
};
fn resource_owner(role: Role) -> Resources {
    Resources::new(
        role,
        Ticket {
            epoch: 1,
            peer: PeerId::parse("ab".repeat(32)).unwrap(),
        },
    )
    .unwrap()
}
fn transfer(source: &mut Multiplexer, target: &mut Multiplexer) -> bool {
    let Some(frame) = source.next_frame().unwrap() else {
        return false;
    };
    let header = frame.header;
    let delivered = target.receive(header).unwrap();
    if header.kind == Kind::Data {
        assert!(delivered);
        assert_eq!(header.channel, Channel::Display);
        assert_eq!(frame.bytes, [42]);
    }
    source.finish(true).unwrap();
    if delivered {
        target.release(header.channel, header.length).unwrap();
    }
    true
}
fn exchange(left: &mut Multiplexer, right: &mut Multiplexer) {
    for _ in 0..24 {
        let moved_left = transfer(left, right);
        let moved_right = transfer(right, left);
        if !moved_left && !moved_right {
            return;
        }
    }
    panic!("bounded metadata exchange did not settle");
}
#[test]
fn t851_crossing_commit_drains_real_credits_and_preserves_display() {
    let (mut host, mut tablet) = (resource_owner(Role::Host), resource_owner(Role::Tablet));
    let mut left = Multiplexer::new(1, 1, 64, 4, 64).unwrap();
    let mut right = Multiplexer::new(1, 1, 64, 4, 64).unwrap();
    left.activate(Channel::Display, 1).unwrap();
    right.activate(Channel::Display, 1).unwrap();
    exchange(&mut left, &mut right);
    let h = host.begin(Resource::Camera, 1, 0, 100).unwrap();
    let t = tablet.offer(h.key, 2, 0, 100).unwrap();
    tablet.prepared(&t, 1).unwrap();
    host.prepared(&h, 2).unwrap();
    host.start_commit(&h, 3).unwrap();
    tablet.revoke(&t).unwrap();
    assert_eq!(host.commit(&h, 4).unwrap(), Install::Activate);
    left.activate(Channel::Camera, h.key.generation).unwrap();
    assert_eq!(tablet.commit(&t, 5).unwrap(), Install::Drain);
    right.close(Channel::Camera, Kind::Reset).unwrap();
    assert!(right.offer(Channel::Camera, vec![1]).is_err());
    // Reserve the host's credit before it observes tablet's cancellation.
    let credit = left.next_frame().unwrap().unwrap().header;
    assert_eq!(credit.kind, Kind::Credit);
    assert_eq!(credit.channel, Channel::Camera);
    assert!(!right.receive(credit).unwrap());
    left.finish(true).unwrap();
    host.peer_revoke(&h).unwrap();
    left.close(Channel::Camera, Kind::Reset).unwrap();
    host.revoke_written(&h).unwrap();
    tablet.peer_revoke(&t).unwrap();
    exchange(&mut left, &mut right);
    assert!(!left.retired(Channel::Camera));
    assert!(!right.retired(Channel::Camera));
    left.offer(Channel::Display, vec![42]).unwrap();
    exchange(&mut left, &mut right);
    left.consumer_retired(Channel::Camera, 1).unwrap();
    right.consumer_retired(Channel::Camera, 1).unwrap();
    assert!(left.retired(Channel::Camera));
    assert!(right.retired(Channel::Camera));
    host.channels_retired(&h).unwrap();
    tablet.channels_retired(&t).unwrap();
    tablet.native_retired(&t).unwrap();
    tablet.acknowledge(&t).unwrap();
    host.acknowledge(&h).unwrap();
    host.native_retired(&h).unwrap();
    host.finish(&h).unwrap();
    tablet.finish(&t).unwrap();
    let replacement = host.begin(Resource::Camera, 1, 10, 100).unwrap();
    assert_eq!(replacement.key.generation, 2);
    tablet.offer(replacement.key, 2, 10, 100).unwrap();
    left.replace_channel(Channel::Camera, 2).unwrap();
    right.replace_channel(Channel::Camera, 2).unwrap();
    assert!(left.offer(Channel::Camera, vec![1]).is_err());
    left.offer(Channel::Display, vec![42]).unwrap();
    exchange(&mut left, &mut right);
}
