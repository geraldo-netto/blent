use blent_config::connection::pairing::{Pairing, PeerId, Phase, Ticket};
fn peer(c: char) -> PeerId {
    PeerId::parse(c.to_string().repeat(64)).unwrap()
}
#[test]
fn t846_pairing_approval_requires_current_tls_proof_and_retires_before_replacement() {
    let mut owner = Pairing::default();
    assert_eq!(owner.phase(), Phase::Idle);
    let ticket = owner.begin(peer('a'), false, 10, 100).unwrap();
    assert_eq!(owner.phase(), Phase::Pending);
    assert!(owner.begin(peer('b'), true, 10, 100).is_err());
    assert!(owner.authenticated(&ticket, &peer('a'), 11).is_err());
    assert!(owner
        .approve(
            &Ticket {
                epoch: 99,
                ..ticket.clone()
            },
            true,
            11
        )
        .is_err());
    owner.approve(&ticket, true, 11).unwrap();
    assert_eq!(owner.phase(), Phase::Approved);
    assert!(owner.authenticated(&ticket, &peer('b'), 12).is_err());
    let lease = owner.authenticated(&ticket, &peer('a'), 12).unwrap();
    assert!(lease.remember);
    assert!(owner.current(&lease));
    assert!(!owner.current(&blent_config::connection::pairing::Lease {
        remember: false,
        ..lease.clone()
    }));
    assert!(owner.authenticated(&ticket, &peer('a'), 13).is_err());
    assert!(!owner.expire(10000).unwrap());
    assert!(!owner.revoke(&peer('b')));
    assert!(owner.retired(&ticket).is_err());
    assert!(owner.revoke(&peer('a')));
    assert!(!owner.current(&lease));
    assert_eq!(owner.phase(), Phase::Closing);
    assert!(owner.begin(peer('a'), true, 20, 100).is_err());
    assert!(owner
        .retired(&Ticket {
            epoch: 99,
            ..ticket.clone()
        })
        .is_err());
    owner.retired(&ticket).unwrap();
    assert!(!owner.revoke(&peer('a')));
    assert!(!owner.cancel(&ticket));
    let replacement = owner.begin(peer('a'), true, 20, 100).unwrap();
    assert_ne!(replacement, ticket);
    assert!(!owner.cancel(&ticket));
    assert!(owner.approve(&ticket, true, 21).is_err());
    assert!(!owner.current(&lease));
    let next = owner.authenticated(&replacement, &peer('a'), 22).unwrap();
    assert!(!next.remember);
    assert!(owner.current(&next));
}
#[test]
fn t846_pairing_timeout_boundaries_and_bounded_stale_callback_fuzz() {
    let mut owner = Pairing::default();
    assert!(!owner.expire(0).unwrap());
    for (now, timeout) in [(-1, 1), (0, 0), (0, -1), (0, 600001), (i64::MAX, 1)] {
        assert!(owner.begin(peer('a'), false, now, timeout).is_err());
    }
    assert!(owner.expire(-1).is_err());
    for round in 0..1024 {
        let ticket = owner.begin(peer('a'), round % 2 == 0, 10, 2).unwrap();
        assert!(!owner.expire(11).unwrap());
        assert!(owner.approve(&ticket, true, -1).is_err());
        assert!(owner.authenticated(&ticket, &peer('a'), -1).is_err());
        assert!(owner.approve(&ticket, true, 12).is_err());
        assert!(owner.authenticated(&ticket, &peer('a'), 12).is_err());
        assert!(owner.expire(12).unwrap());
        assert_eq!(owner.phase(), Phase::Closing);
        assert!(!owner.expire(13).unwrap());
        assert!(owner.authenticated(&ticket, &peer('a'), 11).is_err());
        owner.retired(&ticket).unwrap();
        assert!(!owner.cancel(&ticket));
    }
}
#[test]
fn t846_peer_fingerprint_invalid_boundaries_and_mutations() {
    assert_eq!(peer('f').as_str(), "f".repeat(64));
    for size in [0, 1, 63, 65, 4096] {
        assert!(PeerId::parse("a".repeat(size)).is_err());
    }
    for value in 0..=255u8 {
        let mut input = "a".repeat(63);
        input.push(char::from(value));
        let valid = value.is_ascii_digit() || (b'a'..=b'f').contains(&value);
        assert_eq!(PeerId::parse(input).is_ok(), valid);
    }
}

#[test]
fn t846_untrusted_identity_header_is_bounded_and_role_specific() {
    use blent_config::connection::hello::{Hello, HEADER_BYTES};
    for length in [1, 255, 256, 4096] {
        for host in [false, true] {
            let hello = Hello {
                host,
                certificate_bytes: length,
            };
            let wire = hello.encode().unwrap();
            assert_eq!(&wire[..5], b"BLPA\x01");
            assert_eq!(wire[5], u8::from(host));
            assert_eq!(Hello::decode(&wire, host).unwrap(), hello);
            assert!(Hello::decode(&wire, !host).is_err());
        }
    }
    for length in [0, 4097, u16::MAX] {
        assert!(Hello {
            host: true,
            certificate_bytes: length
        }
        .encode()
        .is_err());
    }
    for size in [0, 1, 7, 9, 4096] {
        assert!(Hello::decode(&vec![0; size], true).is_err());
    }
    let wire = Hello {
        host: true,
        certificate_bytes: 4096,
    }
    .encode()
    .unwrap();
    for position in 0..HEADER_BYTES {
        for value in 0..=255u8 {
            let mut changed = wire;
            changed[position] = value;
            if let Ok(hello) = Hello::decode(&changed, true) {
                assert_eq!(hello.encode().unwrap(), changed);
            }
        }
    }
}

#[test]
fn t846_setup_readiness_has_exact_role_version_and_extent() {
    use blent_config::connection::ready;
    for host in [true, false] {
        let marker = ready::encode(host);
        assert_eq!(&marker[..5], b"BLPR\x01");
        assert!(ready::decode(&marker, host).is_ok());
        assert!(ready::decode(&marker, !host).is_err());
        for length in [0, 1, 7, 9, 4096] {
            assert!(ready::decode(&vec![0; length], host).is_err());
        }
        for index in 0..ready::BYTES {
            for value in 0..=255u8 {
                let mut changed = marker;
                changed[index] = value;
                assert_eq!(ready::decode(&changed, host).is_ok(), changed == marker);
            }
        }
    }
}
