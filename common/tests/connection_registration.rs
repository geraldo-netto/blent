use blent_config::connection::{
    management::{Audio, Camera, Credential, Profile, Resource},
    pairing::PeerId,
    registration::{Message, Rejection, State, HEADER_BYTES, MAX_PACKET},
    routes::RouteId,
    wire::MAX_COUNTER,
};

fn profiles() -> Vec<(Resource, Profile)> {
    vec![
        (Resource::DisplaySession, Profile::Display),
        (
            Resource::Camera,
            Profile::Camera(Camera {
                width: 640,
                height: 480,
                fps: 30,
                bitrate: 4000,
                min_bitrate: 256,
                freshness_ms: 50,
                lens: 1,
                background: true,
                adaptive: true,
            }),
        ),
        (
            Resource::Microphone,
            Profile::Audio(Audio {
                processing: 1,
                buffer_ms: 20,
                background: false,
            }),
        ),
        (
            Resource::Speakers,
            Profile::Audio(Audio {
                processing: 2,
                buffer_ms: 200,
                background: true,
            }),
        ),
    ]
}
fn registration(resource: Resource, profile: Profile, request: u64) -> Message {
    Message::Register {
        request,
        peer: PeerId::parse("ab".repeat(32)).unwrap(),
        resource,
        credential: Credential::new(std::array::from_fn(|i| i as u8)),
        profile,
    }
}
fn route() -> RouteId {
    RouteId {
        owner: 7,
        revision: 9,
    }
}
fn roundtrip(message: Message) {
    let encoded = message.encode().unwrap();
    assert!(encoded.len() <= MAX_PACKET);
    let decoded = Message::decode(&encoded).unwrap();
    assert!(decoded == message);
    assert_eq!(decoded.descriptor_count(), message.descriptor_count());
    assert_eq!(decoded.request(), message.request());
    assert_eq!(decoded.encode().unwrap(), encoded);
    for length in 0..encoded.len() {
        assert!(
            Message::decode(&encoded[..length]).is_err(),
            "T871 truncated extent {length}"
        );
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(Message::decode(&trailing).is_err());
}

#[test]
fn t871_all_typed_messages_roundtrip_and_have_exact_descriptor_counts() {
    for (resource, profile) in profiles() {
        let message = registration(resource, profile, MAX_COUNTER);
        assert_eq!(
            message.descriptor_count(),
            if resource == Resource::DisplaySession {
                2
            } else {
                1
            }
        );
        roundtrip(message);
    }
    roundtrip(Message::Unregister {
        request: 1,
        route: route(),
    });
    roundtrip(Message::Registered {
        request: MAX_COUNTER,
        route: route(),
    });
    for value in 0..=4 {
        roundtrip(Message::Status {
            request: 2,
            route: route(),
            state: State::try_from(value).unwrap(),
        });
    }
    for value in 0..=5 {
        roundtrip(Message::Rejected {
            request: 3,
            reason: Rejection::try_from(value).unwrap(),
        });
    }
    for value in 0u8..=255 {
        assert_eq!(State::try_from(value).is_ok(), value <= 4);
        assert_eq!(Rejection::try_from(value).is_ok(), value <= 5);
    }
}

#[test]
fn t871_independent_fixed_layout_and_local_request_identity() {
    let message = registration(
        Resource::DisplaySession,
        Profile::Display,
        0x0102030405060708,
    );
    let bytes = message.encode().unwrap();
    assert_eq!(
        &bytes[..16],
        b"BLRG\x01\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08"
    );
    assert_eq!(&bytes[16..80], "ab".repeat(32).as_bytes());
    assert_eq!(bytes[80], 1);
    assert_eq!(&bytes[81..], &(0u8..32).collect::<Vec<_>>());
    assert_eq!(bytes.len(), 113);
    let registered = Message::Registered {
        request: 1,
        route: RouteId {
            owner: 2,
            revision: 3,
        },
    }
    .encode()
    .unwrap();
    assert_eq!(&registered[16..], b"\0\0\0\0\0\0\0\x02\0\0\0\0\0\0\0\x03");
    assert_eq!(registered[5], 2);
    assert_eq!(Message::decode(&registered).unwrap().descriptor_count(), 0);
}

#[test]
fn t871_invalid_counters_and_profiles_reject_without_output_mutation() {
    for value in [0, MAX_COUNTER + 1, u64::MAX] {
        assert!(
            registration(Resource::DisplaySession, Profile::Display, value)
                .encode()
                .is_err()
        );
        for route in [
            RouteId {
                owner: value,
                revision: 1,
            },
            RouteId {
                owner: 1,
                revision: value,
            },
        ] {
            assert!(Message::Unregister { request: 1, route }.encode().is_err());
            assert!(Message::Registered { request: 1, route }.encode().is_err());
            assert!(Message::Status {
                request: 1,
                route,
                state: State::Closing
            }
            .encode()
            .is_err());
        }
    }
    for (resource, profile) in profiles() {
        let mut output = [0xa5; 32];
        let size = profile.encode_for(resource, &mut output).unwrap();
        assert_eq!(
            Profile::decode_for(resource, &output[..size]).unwrap(),
            profile
        );
        assert!(Profile::decode_for(resource, &output[..size + 1]).is_err());
        if size > 0 {
            let mut short = vec![0xa5; size - 1];
            assert!(profile.encode_for(resource, &mut short).is_err());
            assert_eq!(short, vec![0xa5; size - 1]);
        }
    }
    let bad = Profile::Audio(Audio {
        processing: 0,
        buffer_ms: 201,
        background: true,
    });
    assert!(registration(Resource::Microphone, bad, 1).encode().is_err());
    let mut untouched = [0xa5; 32];
    assert!(Profile::Display
        .encode_for(Resource::Camera, &mut untouched)
        .is_err());
    assert_eq!(untouched, [0xa5; 32]);
}

#[test]
fn t871_malformed_header_body_and_route_values_reject() {
    let display = registration(Resource::DisplaySession, Profile::Display, 1)
        .encode()
        .unwrap();
    for index in 0..8 {
        let mut bytes = display.clone();
        bytes[index] = 255;
        assert!(Message::decode(&bytes).is_err());
    }
    for value in [0, MAX_COUNTER + 1, u64::MAX] {
        let mut bytes = display.clone();
        bytes[8..16].copy_from_slice(&value.to_be_bytes());
        assert!(Message::decode(&bytes).is_err());
        let base = Message::Registered {
            request: 1,
            route: route(),
        }
        .encode()
        .unwrap();
        for offset in [16, 24] {
            let mut bad = base.clone();
            bad[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
            assert!(Message::decode(&bad).is_err());
        }
    }
    for index in [16, 79, 80] {
        let mut bytes = display.clone();
        bytes[index] = 255;
        assert!(Message::decode(&bytes).is_err());
    }
    for message in [
        Message::Status {
            request: 1,
            route: route(),
            state: State::Active,
        },
        Message::Rejected {
            request: 1,
            reason: Rejection::Busy,
        },
    ] {
        let mut bytes = message.encode().unwrap();
        *bytes.last_mut().unwrap() = 255;
        assert!(Message::decode(&bytes).is_err());
    }
    assert!(Message::decode(&vec![0; MAX_PACKET + 1]).is_err());
    assert_eq!(HEADER_BYTES, 16);
}

#[test]
fn t871_bounded_packet_mutations_preserve_canonical_models_or_reject() {
    let bases = profiles()
        .into_iter()
        .map(|(r, p)| registration(r, p, 1).encode().unwrap())
        .collect::<Vec<_>>();
    let mut random = 0x128ab391u32;
    for index in 0..4096 {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let mut bytes = bases[index % bases.len()].clone();
        let offset = random as usize % bytes.len();
        bytes[offset] ^= (random >> 24) as u8;
        if let Ok(message) = Message::decode(&bytes) {
            assert_eq!(message.encode().unwrap(), bytes);
            assert!((1..=MAX_COUNTER).contains(&message.request()));
            assert!((1..=2).contains(&message.descriptor_count()));
        }
    }
    for size in 0..=MAX_PACKET {
        assert!(Message::decode(&vec![0; size]).is_err());
    }
}
