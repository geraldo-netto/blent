use super::*;
use crate::connection::{registration::Message, wire::MAX_COUNTER};

fn peer() -> PeerId {
    PeerId::parse("ab".repeat(32)).unwrap()
}
fn ticket() -> Ticket {
    Ticket {
        epoch: 2,
        peer: peer(),
    }
}
fn commands() -> Vec<Command> {
    vec![
        Command::Inspect,
        Command::Connect {
            revision: 1,
            target: 2,
        },
        Command::Decide {
            connection: 3,
            ticket: ticket(),
            allow: true,
            remember: true,
        },
        Command::Decide {
            connection: 3,
            ticket: ticket(),
            allow: false,
            remember: false,
        },
        Command::Disconnect {
            connection: 3,
            ticket: Some(ticket()),
        },
        Command::Disconnect {
            connection: 3,
            ticket: None,
        },
        Command::Revoke { peer: peer() },
    ]
}
fn notices() -> Vec<Notice> {
    let mut values = vec![
        Notice::Begin {
            revision: 1,
            local: peer(),
        },
        Notice::Device {
            revision: 1,
            target: 2,
            vendor: u16::MAX,
            product: 0,
            accessory: false,
            names: Default::default(),
            connection: None,
        },
        Notice::Device {
            revision: 1,
            target: 3,
            vendor: 0x18d1,
            product: 0x2d00,
            accessory: true,
            names: Default::default(),
            connection: None,
        },
        Notice::Saved {
            revision: 1,
            peer: peer(),
        },
        Notice::End { revision: 1 },
    ];
    for state in [
        State::Discovering,
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ] {
        let known = state != State::Discovering;
        values.push(Notice::Link {
            revision: 1,
            connection: 3,
            ticket: known.then(ticket),
            state,
            trusted: known,
        });
    }
    values
}

#[test]
fn t877_local_control_roundtrips_and_never_accepts_descriptors() {
    for command in commands() {
        assert_eq!(
            Command::decode(&command.encode().unwrap()).unwrap(),
            command
        );
        let message = Message::Control {
            request: 1,
            command,
        };
        assert_eq!(message.descriptor_count(), 0);
        assert!(Message::decode(&message.encode().unwrap()).unwrap() == message);
    }
    for notice in notices() {
        assert_eq!(Notice::decode(&notice.encode().unwrap()).unwrap(), notice);
        let message = Message::Report {
            request: MAX_COUNTER,
            notice,
        };
        assert_eq!(message.descriptor_count(), 0);
        assert!(Message::decode(&message.encode().unwrap()).unwrap() == message);
    }
}

#[test]
fn t877_truncation_trailing_and_unknown_operations_rejected() {
    for command in commands() {
        let mut bytes = command.encode().unwrap();
        for end in 0..bytes.len() {
            assert!(Command::decode(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert!(Command::decode(&bytes).is_err());
        bytes[0] = 255;
        assert!(Command::decode(&bytes).is_err());
    }
    for notice in notices() {
        let mut bytes = notice.encode().unwrap();
        for end in 0..bytes.len() {
            assert!(Notice::decode(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert!(Notice::decode(&bytes).is_err());
        bytes[0] = 255;
        assert!(Notice::decode(&bytes).is_err());
    }
}

#[test]
fn t877_counter_range_and_invalid_decisions_reject_before_publication() {
    for value in [0, 1, MAX_COUNTER, MAX_COUNTER + 1, u64::MAX] {
        let valid = (1..=MAX_COUNTER).contains(&value);
        assert_eq!(
            Command::Connect {
                revision: value,
                target: 1
            }
            .encode()
            .is_ok(),
            valid
        );
        assert_eq!(
            Command::Connect {
                revision: 1,
                target: value
            }
            .encode()
            .is_ok(),
            valid
        );
        assert_eq!(
            Command::Disconnect {
                connection: value,
                ticket: None
            }
            .encode()
            .is_ok(),
            valid
        );
        let mut proof = ticket();
        proof.epoch = value;
        assert_eq!(
            Command::Decide {
                connection: 1,
                ticket: proof,
                allow: true,
                remember: false
            }
            .encode()
            .is_ok(),
            valid
        );
        assert_eq!(Notice::End { revision: value }.encode().is_ok(), valid);
    }
    assert!(Command::Decide {
        connection: 1,
        ticket: ticket(),
        allow: false,
        remember: true
    }
    .encode()
    .is_err());
    let mut bad = Command::Decide {
        connection: 1,
        ticket: ticket(),
        allow: false,
        remember: false,
    }
    .encode()
    .unwrap();
    *bad.last_mut().unwrap() = 1;
    assert!(Command::decode(&bad).is_err());
}

#[test]
fn t877_state_ticket_and_boolean_contracts_reject_inconsistent_reports() {
    for state in [
        State::Discovering,
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ] {
        for known in [false, true] {
            for trusted in [false, true] {
                let notice = Notice::Link {
                    revision: 1,
                    connection: 2,
                    ticket: known.then(ticket),
                    state,
                    trusted,
                };
                let valid_state = match state {
                    State::Discovering => !known,
                    State::Pending | State::Approved | State::Authenticated => known,
                    _ => true,
                };
                assert_eq!(notice.encode().is_ok(), valid_state && (!trusted || known));
            }
        }
    }
    let mut bytes = Notice::Link {
        revision: 1,
        connection: 2,
        ticket: Some(ticket()),
        state: State::Pending,
        trusted: false,
    }
    .encode()
    .unwrap();
    bytes[17] = 255;
    assert!(Notice::decode(&bytes).is_err());
    bytes[17] = State::Discovering as u8;
    assert!(Notice::decode(&bytes).is_err());
    for value in 2..=255u8 {
        let bytes = [3, 0, 0, 0, 0, 0, 0, 0, 1, value];
        assert!(Command::decode(&bytes).is_err());
    }
}

#[test]
fn t877_mutations_and_arbitrary_bounded_inputs_are_canonical_or_rejected() {
    let mut samples: Vec<Vec<u8>> = commands().iter().map(|v| v.encode().unwrap()).collect();
    samples.extend(notices().iter().map(|v| v.encode().unwrap()));
    let mut seed = 0x17f493u32;
    for length in 0..=256 {
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *byte = (seed >> 24) as u8;
        }
        samples.push(bytes);
    }
    for sample in samples {
        for index in 0..sample.len() {
            let mut bytes = sample.clone();
            bytes[index] ^= 0xff;
            if let Ok(command) = Command::decode(&bytes) {
                assert_eq!(command.encode().unwrap(), bytes);
            }
            if let Ok(notice) = Notice::decode(&bytes) {
                assert_eq!(notice.encode().unwrap(), bytes);
            }
        }
    }
}

#[test]
fn t1336_device_name_and_model_survive_control_wire() {
    // Independent current-format record: IDs/flag, then two u16-length UTF-8 names.
    let mut bytes = vec![1];
    bytes.extend_from_slice(&1u64.to_be_bytes());
    bytes.extend_from_slice(&2u64.to_be_bytes());
    bytes.extend_from_slice(&[0x18, 0xd1, 0x2d, 1, 1]);
    for value in ["Unisoc", "RugKing Pad 2 Pro"] {
        bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes.push(0); // No owning connection.
    let notice = Notice::decode(&bytes).expect("T1336 manufacturer/model must reach the GUI");
    assert_eq!(notice.encode().unwrap(), bytes);
}

#[test]
fn t1336_maximum_names_fit_registration_and_reject_malformed_fields() {
    let names = DeviceNames::new("a".repeat(256), "é".repeat(128)).unwrap();
    let notice = Notice::Device {
        revision: 1,
        target: 2,
        vendor: 0x18d1,
        product: 0x2d01,
        accessory: true,
        names,
        connection: None,
    };
    let message = Message::Report {
        request: 1,
        notice: notice.clone(),
    };
    assert!(Message::decode(&message.encode().unwrap()).unwrap() == message);
    let bytes = notice.encode().unwrap();
    for end in 0..bytes.len() {
        assert!(Notice::decode(&bytes[..end]).is_err());
    }
    for invalid in [0, 0xff, b'\n'] {
        let mut corrupt = bytes.clone();
        corrupt[24] = invalid;
        assert!(Notice::decode(&corrupt).is_err());
    }
    let mut corrupt = bytes;
    corrupt[22..24].copy_from_slice(&257u16.to_be_bytes());
    assert!(Notice::decode(&corrupt).is_err());
}

#[test]
fn t1337_device_owner_counter_is_bounded_and_canonical() {
    for connection in [
        None,
        Some(0),
        Some(1),
        Some(MAX_COUNTER),
        Some(MAX_COUNTER + 1),
    ] {
        let mut notice = notices().remove(1);
        if let Notice::Device {
            connection: owner, ..
        } = &mut notice
        {
            *owner = connection;
        }
        let encoded = notice.encode();
        assert_eq!(
            encoded.is_ok(),
            connection.is_none_or(|value| (1..=MAX_COUNTER).contains(&value))
        );
        if let Ok(bytes) = encoded {
            assert_eq!(Notice::decode(&bytes).unwrap(), notice);
        }
    }
}
