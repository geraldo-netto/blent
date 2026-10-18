//! T1016 bounded joined acceptance; original receipts never substitute for peers.
use super::*;
use crate::{
    camera_registration::test_support::{authenticated, profile, Rig},
    media_registration::MediaRegistration,
};
use blent_config::connection::{
    management::{Profile, Resource},
    registration_client::Request,
};

fn settle(fixture: &ActivityFixture, broker: &Route, producer: usize) {
    if producer == 0 {
        broker.done.store(true, Ordering::SeqCst);
        return;
    }
    let participant = &fixture.participants[producer - 1];
    assert!(participant.acknowledge(&participant.pending().unwrap()));
}
fn cycle(fixture: &mut ActivityFixture, order: [usize; 3], revision: u64, now: Instant) {
    let broker = fixture.host.client();
    let original = fixture.host.attachment.lease();
    let count = fixture.count();
    fixture.observe(revision, true);
    fixture.poll(now);
    assert!(!original.apply(|| ()));
    fixture.observe(revision + 1, false);
    for (step, producer) in order.into_iter().enumerate() {
        fixture.poll(now + Duration::from_secs(2));
        assert_eq!(
            fixture.count(),
            count,
            "T1016 premature native/broker reuse"
        );
        settle(fixture, &broker, producer);
        fixture.poll(now + Duration::from_secs(2));
        if step < 2 {
            assert_eq!(fixture.count(), count);
        }
    }
    fixture.poll(now + Duration::from_secs(3));
    assert_eq!(fixture.count(), count + 1);
    assert!(fixture.participants[0].source().begin().is_none());
    fixture.host.client().set(Phase::Active);
    fixture.poll(now + Duration::from_secs(3));
    assert!(fixture.participants[0].source().begin().is_some());
    assert_eq!(fixture.host.native.closes.load(Ordering::SeqCst), 0);
}

#[test]
fn t1016_all_original_retirement_orders_preserve_selected_intent_across_repeated_cycles() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let now = Instant::now();
        let mut fixture = ActivityFixture::new(false);
        fixture.active(now);
        let ticket = fixture.host.session.ticket();
        for turn in 0..16 {
            cycle(
                &mut fixture,
                order,
                2 + 2 * turn,
                now + Duration::from_secs(10 * turn),
            );
            assert_eq!(fixture.host.session.ticket(), ticket);
            assert!(fixture.host.session.status().is_some());
        }
        fixture.host.session.close();
        fixture.acknowledge();
        fixture.host.client().done.store(true, Ordering::SeqCst);
        fixture.host.native.done.store(true, Ordering::SeqCst);
        fixture.poll(now + Duration::from_secs(200));
        assert!(fixture.host.session.retired());
    }
}

fn auxiliary(resource: Resource) -> (Rig, MediaRegistration) {
    use blent_config::audio::{AudioProfile, Direction};
    let profile = match resource {
        Resource::Camera => Profile::Camera(profile().invitation_profile().unwrap()),
        Resource::Microphone => Profile::Audio(
            AudioProfile::new(Direction::Microphone)
                .invitation_profile()
                .unwrap()
                .1,
        ),
        Resource::Speakers => Profile::Audio(
            AudioProfile::new(Direction::Speakers)
                .invitation_profile()
                .unwrap()
                .1,
        ),
        Resource::DisplaySession => panic!("T1016 auxiliary fixture requires media resource"),
    };
    let rig = authenticated();
    let mut owner = rig
        .media(Request {
            peer: peer(2),
            resource,
            credential: Credential::new([91; 32]),
            profile,
        })
        .unwrap();
    owner.poll(rig.now).unwrap();
    rig.client().set(Phase::Active);
    owner.poll(rig.now).unwrap();
    (rig, owner)
}
#[test]
fn t1016_display_pause_does_not_revoke_camera_microphone_or_speaker_owners() {
    let now = Instant::now();
    let mut fixture = ActivityFixture::new(false);
    let mut media = [
        auxiliary(Resource::Camera),
        auxiliary(Resource::Microphone),
        auxiliary(Resource::Speakers),
    ];
    fixture.active(now);
    for turn in 0..16 {
        cycle(
            &mut fixture,
            [2, 0, 1],
            2 + 2 * turn,
            now + Duration::from_secs(10 * turn),
        );
        for (rig, owner) in &mut media {
            owner
                .poll(rig.now + Duration::from_secs(10 * turn))
                .unwrap();
            assert_eq!(owner.state(), crate::media_registration::State::Active);
            assert!(!rig.client().closed.load(Ordering::SeqCst));
            assert!(!rig.source.closed.load(Ordering::SeqCst));
            assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
        }
    }
    fixture.host.session.close();
    for (rig, owner) in &mut media {
        assert!(!rig.client().closed.load(Ordering::SeqCst));
        owner.close();
        assert!(!owner.retired());
        rig.complete();
        assert!(owner.retired());
    }
}
