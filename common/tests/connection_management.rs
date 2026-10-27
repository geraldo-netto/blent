use blent_config::connection::management::*;
fn key(resource: Resource) -> Key {
    Key {
        resource,
        request: 7,
        generation: 9,
    }
}
fn camera() -> Camera {
    Camera {
        width: 1920,
        height: 1080,
        fps: 30,
        bitrate: 20000,
        min_bitrate: 256,
        freshness_ms: 150,
        lens: 1,
        background: true,
        adaptive: true,
    }
}
fn audio() -> Audio {
    Audio {
        processing: 2,
        buffer_ms: 20,
        background: false,
    }
}
fn messages() -> Vec<Message> {
    vec![
        Message::Capabilities(15),
        Message::Offer(
            key(Resource::DisplaySession),
            Credential::new([42; 32]),
            Profile::Display,
        ),
        Message::Offer(
            key(Resource::Camera),
            Credential::new([43; 32]),
            Profile::Camera(camera()),
        ),
        Message::Offer(
            key(Resource::Microphone),
            Credential::new([44; 32]),
            Profile::Audio(audio()),
        ),
        Message::Offer(
            key(Resource::Speakers),
            Credential::new([45; 32]),
            Profile::Audio(audio()),
        ),
        Message::Prepared(key(Resource::Camera)),
        Message::Commit(key(Resource::Camera)),
        Message::Result(key(Resource::Camera), Outcome::Admitted),
        Message::Revoke(key(Resource::Camera), Reason::Requested),
    ]
}
fn bytes(message: &Message) -> Vec<u8> {
    let mut out = [0; MAX_RECORD];
    let size = message.encode(&mut out).unwrap();
    out[..size].to_vec()
}
#[test]
fn t850_typed_records_roundtrip_and_decode_every_split() {
    for message in messages() {
        let wire = bytes(&message);
        assert!(Message::decode(&wire).unwrap() == message);
        for split in 0..=wire.len() {
            let mut decoder = Decoder::new();
            let (count, first) = decoder.feed(&wire[..split]).unwrap();
            assert_eq!(count, split);
            let (count, second) = decoder.feed(&wire[split..]).unwrap();
            assert_eq!(count, wire.len() - split);
            assert!(first.or(second).unwrap() == message);
            decoder.end().unwrap();
            assert!(decoder.feed(&[]).is_err());
        }
        for cut in 1..wire.len() {
            let mut decoder = Decoder::new();
            decoder.feed(&wire[..cut]).unwrap();
            assert!(decoder.end().is_err());
            assert!(decoder.feed(&wire).is_err());
        }
    }
}
#[test]
fn t850_coalesced_input_consumes_exactly_one_record_without_growth() {
    let wire = bytes(&messages()[1]);
    let large = wire.repeat(4096);
    let mut decoder = Decoder::default();
    let mut offset = 0;
    while offset < large.len() {
        let (count, message) = decoder.feed(&large[offset..]).unwrap();
        assert_eq!(count, wire.len());
        assert!(message.is_some());
        offset += count;
    }
    decoder.end().unwrap();
    assert!(decoder.end().is_err());
    let mut empty = Decoder::new();
    assert!(empty.feed(&[]).unwrap().1.is_none());
    empty.end().unwrap();
}
#[test]
fn t850_invalid_wire_poisoning_and_bounded_mutations() {
    let wire = bytes(&messages()[2]);
    for position in [0, 4, 5, 6, 7, 8, 16, 24] {
        let mut bad = wire.clone();
        bad[position] = 255;
        assert!(Message::decode(&bad).is_err());
        let mut decoder = Decoder::new();
        assert!(decoder.feed(&bad).is_err());
        assert!(decoder.feed(&wire).is_err());
        assert!(decoder.end().is_err());
    }
    for size in 0..wire.len() {
        assert!(Message::decode(&wire[..size]).is_err());
    }
    let mut extra = wire.clone();
    extra.push(0);
    assert!(Message::decode(&extra).is_err());
    for value in 0..4096usize {
        let mut candidate = wire.clone();
        let index = value % candidate.len();
        candidate[index] = (value / 7) as u8;
        if let Ok(decoded) = Message::decode(&candidate) {
            assert_eq!(bytes(&decoded), candidate);
        }
    }
}
#[test]
fn t850_invalid_models_reject_before_modifying_output() {
    let mut invalid = vec![
        Message::Capabilities(16),
        Message::Offer(
            key(Resource::Camera),
            Credential::new([1; 32]),
            Profile::Display,
        ),
    ];
    for value in [0, u64::MAX] {
        invalid.push(Message::Prepared(Key {
            request: value,
            ..key(Resource::Camera)
        }));
        invalid.push(Message::Commit(Key {
            generation: value,
            ..key(Resource::Camera)
        }));
    }
    for width in [0, 159, 161, 1921, u16::MAX] {
        invalid.push(Message::Offer(
            key(Resource::Camera),
            Credential::new([1; 32]),
            Profile::Camera(Camera { width, ..camera() }),
        ));
    }
    for profile in [
        Camera {
            height: 119,
            ..camera()
        },
        Camera {
            height: 121,
            ..camera()
        },
        Camera {
            height: 1082,
            ..camera()
        },
        Camera { fps: 4, ..camera() },
        Camera {
            fps: 31,
            ..camera()
        },
        Camera {
            bitrate: 255,
            ..camera()
        },
        Camera {
            bitrate: 20001,
            ..camera()
        },
        Camera {
            min_bitrate: 255,
            ..camera()
        },
        Camera {
            min_bitrate: 20001,
            ..camera()
        },
        Camera {
            freshness_ms: 49,
            ..camera()
        },
        Camera {
            freshness_ms: 2001,
            ..camera()
        },
        Camera {
            lens: 2,
            ..camera()
        },
    ] {
        invalid.push(Message::Offer(
            key(Resource::Camera),
            Credential::new([1; 32]),
            Profile::Camera(profile),
        ));
    }
    for profile in [
        Audio {
            processing: 0,
            ..audio()
        },
        Audio {
            processing: 3,
            ..audio()
        },
        Audio {
            buffer_ms: 19,
            ..audio()
        },
        Audio {
            buffer_ms: 21,
            ..audio()
        },
        Audio {
            buffer_ms: 201,
            ..audio()
        },
    ] {
        invalid.push(Message::Offer(
            key(Resource::Speakers),
            Credential::new([1; 32]),
            Profile::Audio(profile),
        ));
    }
    for message in invalid {
        let mut out = [123; MAX_RECORD];
        assert!(message.encode(&mut out).is_err());
        assert_eq!(out, [123; MAX_RECORD]);
    }
    let mut short = [123; 27];
    assert!(messages()[0].encode(&mut short).is_err());
    assert_eq!(short, [123; 27]);
    assert_eq!(Credential::new([42; 32]).as_bytes(), &[42; 32]);
}

fn fixture_bytes(value: &serde_json::Value) -> Vec<u8> {
    value["hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn t850_shared_vectors_fix_cross_language_wire_and_rejections() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../testdata/connection-management-v1.json")).unwrap();
    for value in fixture["valid"].as_array().unwrap() {
        let wire = fixture_bytes(value);
        let message = Message::decode(&wire).unwrap();
        assert_eq!(bytes(&message), wire, "{}", value["name"]);
        let mut decoder = Decoder::new();
        let mut result = None;
        for byte in &wire {
            let (count, decoded) = decoder.feed(&[*byte]).unwrap();
            assert_eq!(count, 1);
            if decoded.is_some() {
                assert!(result.is_none());
                result = decoded;
            }
        }
        assert!(result.unwrap() == message);
        decoder.end().unwrap();
        let mut output = [127; MAX_RECORD];
        let size = message.encode(&mut output).unwrap();
        assert!(output[size..].iter().all(|byte| *byte == 127));
    }
    for value in fixture["invalid"].as_array().unwrap() {
        let wire = fixture_bytes(value);
        assert!(Message::decode(&wire).is_err(), "{}", value["name"]);
        let mut decoder = Decoder::new();
        match decoder.feed(&wire) {
            Err(_) => {
                assert!(decoder.feed(&[]).is_err());
                assert!(decoder.end().is_err());
            }
            Ok((consumed, None)) => {
                assert_eq!(consumed, wire.len());
                assert!(decoder.end().is_err());
            }
            Ok((consumed, Some(_))) => assert!(
                consumed < wire.len(),
                "only valid first record plus trailing bytes can complete"
            ),
        }
    }
}
