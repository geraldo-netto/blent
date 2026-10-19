//! T1103: configured original-link dispatch with an independent protocol clock.
use super::*;
use crate::broker::session::{tests::Rig, Event as SessionEvent};
use blent_config::connection::{
    management::{Credential, Profile, Resource as MediaResource},
    wire::Channel,
};

fn display() -> Command {
    Command::Register(Offer {
        resource: MediaResource::DisplaySession,
        owner: 17,
        credential: Credential::new([9; 32]),
        profile: Profile::Display,
        // No native connector is polled in this admission-only fixture.
        connect: Box::pin(std::future::pending()),
    })
}
fn registered(events: &mut mpsc::Receiver<Event>) -> Lease {
    match events.try_recv().unwrap() {
        Event::Registered { owner, result } => {
            assert_eq!(owner, 17);
            result.unwrap()
        }
        _ => panic!("T1103 missing original registration"),
    }
}
fn closing(events: &[SessionEvent], lease: &Lease) -> bool {
    events.iter().any(
        |event| matches!(event, SessionEvent::State(current, State::Closing) if current == lease),
    )
}
fn independent_camera(rig: &mut Rig, camera: &Lease, now: i64) {
    assert!(rig.host.current(camera));
    rig.peer.offer(Channel::Camera, vec![11; 16]).unwrap();
    rig.pump(now).unwrap();
    let mut bytes = [0; 16];
    let received = rig
        .host
        .read(camera, Channel::Camera, &mut bytes)
        .unwrap()
        .unwrap();
    assert_eq!(received.count, 16);
    assert_eq!(bytes, [11; 16]);
    rig.host
        .consumed(camera, Channel::Camera, received)
        .unwrap();
}
fn selected_timeout(milliseconds: i64) {
    let mut rig = Rig::new();
    let camera = rig.admit(MediaResource::Camera);
    let mut bridges = Bridges::new(4096, Duration::from_secs(5)).unwrap();
    let (output, mut input) = mpsc::channel(4);
    let selected = AdmissionTimeout::new(milliseconds).unwrap();
    dispatch(&mut rig.host, &mut bridges, display(), &output, 3, selected).unwrap();
    let lease = registered(&mut input);
    rig.host.tick(3 + milliseconds - 1).unwrap();
    assert!(!closing(&rig.pump(3 + milliseconds - 1).unwrap(), &lease));
    rig.host.tick(3 + milliseconds).unwrap();
    assert!(closing(&rig.pump(3 + milliseconds).unwrap(), &lease));
    independent_camera(&mut rig, &camera, 3 + milliseconds);
    bridges.disconnect(&mut rig.host).unwrap();
}
#[tokio::test]
async fn t1103_original_dispatch_expiry_preserves_unrelated_active_camera() {
    for milliseconds in [1, 37, 120_000, 600_000] {
        selected_timeout(milliseconds);
    }
}
