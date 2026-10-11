use super::*;
use crate::{
    camera_registration::test_support::{authenticated, profile, view},
    display_test_support::peer,
};
use blent_config::connection::{
    control::State as LinkState,
    management::{Audio, Credential, Profile},
};
use std::sync::atomic::Ordering;

fn request(resource: Resource) -> Request {
    Request {
        peer: peer(2),
        resource,
        credential: Credential::new([0xab; 32]),
        profile: if resource == Resource::Camera {
            Profile::Camera(profile().invitation_profile().unwrap())
        } else {
            Profile::Audio(Audio {
                processing: 1,
                buffer_ms: 40,
                background: false,
            })
        },
    }
}

#[test]
fn t976_all_media_keep_exact_request_and_wait_for_independent_cleanup() {
    for resource in [Resource::Camera, Resource::Microphone, Resource::Speakers] {
        let rig = authenticated();
        let expected = request(resource);
        let mut owner = rig.media(expected.clone()).unwrap();
        assert_eq!(owner.state(), State::Waiting);
        assert_eq!(rig.source.starts.load(Ordering::SeqCst), 0);
        owner.poll(rig.now).unwrap();
        let calls = rig.calls.lock().unwrap();
        assert_eq!(calls.requests.len(), 1);
        assert_eq!(calls.requests[0].peer, expected.peer);
        assert_eq!(calls.requests[0].resource, expected.resource);
        assert_eq!(calls.requests[0].profile, expected.profile);
        assert_eq!(
            calls.requests[0].credential.as_bytes(),
            expected.credential.as_bytes()
        );
        drop(calls);
        rig.client().set(Phase::Active);
        owner.poll(rig.now).unwrap();
        assert_eq!(owner.state(), State::Active);
        owner.close();
        assert_eq!(owner.state(), State::Retiring);
        rig.client().set(Phase::Retired);
        rig.client().done.store(true, Ordering::SeqCst);
        assert!(!owner.retired());
        rig.source.done.store(true, Ordering::SeqCst);
        owner.poll(rig.now).unwrap();
        assert_eq!(owner.state(), State::Retired);
    }
}

#[test]
fn t976_audio_replacements_and_admission_rollback_cannot_revive_original_lifetime() {
    for resource in [Resource::Microphone, Resource::Speakers] {
        for replace_source in [false, true] {
            let rig = authenticated();
            let mut owner = rig.media(request(resource)).unwrap();
            owner.poll(rig.now).unwrap();
            rig.client().set(Phase::Active);
            owner.poll(rig.now).unwrap();
            if replace_source {
                rig.source.status.lock().unwrap().view =
                    Some(view(&[(1, 2, LinkState::Authenticated)]));
            } else {
                rig.client().set(Phase::Queued);
            }
            assert!(owner.poll(rig.now).is_err());
            assert!(rig.client().closed.load(Ordering::SeqCst));
            assert!(rig.source.closed.load(Ordering::SeqCst));
            for _ in 0..64 {
                owner.poll(rig.now).unwrap();
            }
            assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
            assert!(!owner.retired());
            rig.complete();
            assert!(owner.retired());
        }
    }
}

#[test]
fn t976_invalid_or_mismatched_media_profiles_fail_before_any_native_exposure() {
    let rig = authenticated();
    for (resource, expected) in [
        (Resource::Camera, [false, true, false]),
        (Resource::Microphone, [false, false, true]),
        (Resource::Speakers, [false, false, true]),
        (Resource::DisplaySession, [false, false, false]),
    ] {
        let profiles = [
            Profile::Display,
            request(Resource::Camera).profile,
            request(Resource::Speakers).profile,
        ];
        for (profile, expected) in profiles.into_iter().zip(expected) {
            let mut candidate = request(resource);
            candidate.profile = profile;
            assert_eq!(rig.media(candidate).is_ok(), expected);
        }
    }
    for processing in [0, 1, 2, 3, u8::MAX] {
        for buffer_ms in [0, 19, 20, 21, 40, 199, 200, 201, u16::MAX] {
            for resource in [Resource::Microphone, Resource::Speakers] {
                let mut candidate = request(resource);
                candidate.profile = Profile::Audio(Audio {
                    processing,
                    buffer_ms,
                    background: true,
                });
                assert_eq!(
                    rig.media(candidate).is_ok(),
                    (1..=2).contains(&processing)
                        && (20..=200).contains(&buffer_ms)
                        && buffer_ms % 10 == 0
                );
            }
        }
    }
    assert_eq!(rig.source.starts.load(Ordering::SeqCst), 0);
    assert!(rig.calls.lock().unwrap().requests.is_empty());
}
