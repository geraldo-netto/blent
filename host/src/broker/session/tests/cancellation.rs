//! T994: delayed local Cancel after EOF/remote retirement must preserve the link.
use super::*;
use crate::broker::link::cancel_original;

#[tokio::test]
async fn t994_late_local_cancel_preserves_sibling_and_replacement_authority() {
    let mut rig = Rig::new();
    let original = rig.admit(Resource::Speakers);
    let sibling = rig.admit(Resource::Camera);
    rig.host.cancel(&original, Reason::WorkerFailed).unwrap();
    rig.pump(3).unwrap();
    rig.message(Message::Result(original.key, Outcome::Revoked));
    rig.pump(4).unwrap();
    rig.host.native_retired(&original).unwrap();
    assert!(rig.host.slots[3].is_none());
    cancel_original(&mut rig.host, &original)
        .expect("T994 late original cancellation terminated shared connection");
    assert!(rig.host.current(&sibling));
    rig.host
        .offer(&sibling, Channel::Camera, b"unaffected".to_vec())
        .unwrap();
    rig.pump(5).unwrap();
    assert!(rig
        .received
        .iter()
        .any(|(channel, bytes)| *channel == Channel::Camera && bytes == b"unaffected"));
    rig.peer.consumer_retired(Channel::Speaker, 1).unwrap();
    rig.peer.replace_channel(Channel::Speaker, 2).unwrap();
    let replacement = rig.admit(Resource::Speakers);
    assert_eq!(replacement.key.generation, 2);
    cancel_original(&mut rig.host, &original).unwrap();
    assert!(rig.host.current(&replacement));
    assert!(rig.host.cancel(&original, Reason::Requested).is_err());
    assert!(rig.host.native_retired(&original).is_err());
    assert!(rig.host.current(&replacement) && rig.host.current(&sibling));
}
#[tokio::test]
async fn t994_local_cancel_still_revokes_pending_and_current_exact_original() {
    let mut rig = Rig::new();
    let pending = rig.register(Resource::Microphone);
    cancel_original(&mut rig.host, &pending).unwrap();
    assert!(rig.host.slots[2].as_ref().unwrap().closing);
    cancel_original(&mut rig.host, &pending).unwrap();
    let admitted = rig.admit(Resource::Camera);
    assert!(rig.host.current(&admitted));
    cancel_original(&mut rig.host, &admitted).unwrap();
    assert!(!rig.host.current(&admitted));
    assert!(rig.host.slots[1].as_ref().unwrap().closing);
}
#[tokio::test]
async fn t994_stale_local_cancel_never_targets_a_different_full_lease() {
    let mut rig = Rig::new();
    let current = rig.admit(Resource::Camera);
    for field in 0..6 {
        let mut stale = current.clone();
        match field {
            0 => stale.owner += 1,
            1 => stale.key.request += 1,
            2 => stale.key.generation += 1,
            3 => stale.connection.epoch += 1,
            4 => stale.connection.peer = PeerId::parse("cd".repeat(32)).unwrap(),
            _ => stale.key.resource = Resource::Speakers,
        }
        cancel_original(&mut rig.host, &stale).unwrap();
        assert!(rig.host.current(&current));
        assert!(rig.host.cancel(&stale, Reason::Requested).is_err());
    }
}
