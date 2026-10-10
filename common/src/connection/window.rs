//! T875 authenticated v1 runtime-window agreement, before any framed CREDIT.
//! Host proposes its configured capacity; tablet selects the smaller of that
//! proposal and its own capacity. Both directions then use the selected value.
//! These bytes belong inside the mutually authenticated TLS stream. Parsing
//! them does not establish identity, approval or resource authority.
use super::wire::MAX_CREDIT;
use anyhow::{ensure, Result};

pub const BYTES: usize = 12;
pub const MIN_CAPACITY: u32 = 4096;

pub fn validate(capacity: u32) -> Result<()> {
    ensure!(
        (MIN_CAPACITY..=MAX_CREDIT).contains(&capacity),
        "invalid connection receive window"
    );
    Ok(())
}

/// Byte 5 is 1 for host proposal and 0 for tablet selection; 6..8 reserved.
pub fn encode(host: bool, capacity: u32) -> Result<[u8; BYTES]> {
    validate(capacity)?;
    let mut bytes = [b'B', b'L', b'C', b'P', 1, u8::from(host), 0, 0, 0, 0, 0, 0];
    bytes[8..12].copy_from_slice(&capacity.to_be_bytes());
    Ok(bytes)
}

pub fn decode(bytes: &[u8], expect_host: bool) -> Result<u32> {
    ensure!(bytes.len() == BYTES, "invalid window record length");
    ensure!(
        bytes[..8] == [b'B', b'L', b'C', b'P', 1, u8::from(expect_host), 0, 0],
        "invalid window record prefix"
    );
    let capacity = u32::from_be_bytes(bytes[8..12].try_into()?);
    validate(capacity)?;
    Ok(capacity)
}

pub fn select(proposed: u32, local: u32) -> Result<u32> {
    validate(proposed)?;
    validate(local)?;
    Ok(proposed.min(local))
}

pub fn accepted(proposed: u32, selected: u32) -> Result<u32> {
    validate(proposed)?;
    validate(selected)?;
    ensure!(
        selected <= proposed,
        "peer increased connection receive window"
    );
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t875_window_bounds_roles_and_round_trip() {
        let values = [
            0,
            1,
            4095,
            4096,
            4097,
            65536,
            MAX_CREDIT,
            MAX_CREDIT + 1,
            u32::MAX,
        ];
        for capacity in values {
            let valid = (MIN_CAPACITY..=MAX_CREDIT).contains(&capacity);
            assert_eq!(validate(capacity).is_ok(), valid);
            for host in [false, true] {
                let record = encode(host, capacity);
                assert_eq!(record.is_ok(), valid);
                if let Ok(bytes) = record {
                    assert_eq!(decode(&bytes, host).unwrap(), capacity);
                    assert!(decode(&bytes, !host).is_err());
                }
            }
        }
    }

    #[test]
    fn t875_selection_is_symmetric_bounded_and_cannot_expand_proposal() {
        for proposed in [0, 4095, 4096, 262144, MAX_CREDIT, u32::MAX] {
            for local in [0, 4095, 4096, 65536, MAX_CREDIT, u32::MAX] {
                if let Ok(selected) = select(proposed, local) {
                    assert_eq!(selected, proposed.min(local));
                    assert_eq!(accepted(proposed, selected).unwrap(), selected);
                } else {
                    assert!(validate(proposed).is_err() || validate(local).is_err());
                }
                assert_eq!(
                    accepted(proposed, local).is_ok(),
                    validate(proposed).is_ok() && validate(local).is_ok() && local <= proposed
                );
            }
        }
    }

    #[test]
    fn t875_every_prefix_mutation_and_bounded_arbitrary_input_is_checked() {
        let valid = encode(true, MIN_CAPACITY).unwrap();
        for index in 0..8 {
            for value in 0..=255u8 {
                let mut bytes = valid;
                bytes[index] = value;
                assert_eq!(decode(&bytes, true).is_ok(), value == valid[index]);
            }
        }
        let mut seed = 0x17f49u32;
        for length in 0..=64 {
            let mut bytes = vec![0; length];
            for byte in &mut bytes {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *byte = (seed >> 24) as u8;
            }
            assert!(decode(&bytes, true).is_err());
        }
        let mut invalid = valid;
        invalid[8..].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode(&invalid, true).is_err());
    }
}
