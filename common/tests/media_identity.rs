//! T1295: explicit media identities obey the same independent workspace contract.
use blent_config::{
    connection::media::Target,
    tablets::Identity,
    workspace::{Device, Presence},
};

#[test]
fn t1295_explicit_serial_boundaries_agree_with_workspace() {
    for length in [0, 1, 1023, 1024, 1025, 2048] {
        let serial = "a".repeat(length);
        let accepted = (1..=1024).contains(&length);
        assert_eq!(Target::parse(Some(&serial), None).is_ok(), accepted);
        assert_eq!(
            Device::new(
                Identity::Adb(serial),
                String::new(),
                Presence::Unknown,
                false
            )
            .is_ok(),
            accepted
        );
    }
    assert!(Target::parse(None, None).is_ok());
    assert!(Target::parse(Some(&"é".repeat(512)), None).is_ok());
    assert!(Target::parse(Some(&"é".repeat(513)), None).is_err());
}

#[test]
fn t1295_bounded_control_character_property() {
    for value in 0_u8..=255 {
        let serial = format!("prefix{}suffix", char::from(value));
        let accepted = !matches!(value, 0..=31 | 127..=159);
        assert_eq!(
            Target::parse(Some(&serial), None).is_ok(),
            accepted,
            "byte {value}"
        );
        assert_eq!(
            Device::new(
                Identity::Adb(serial),
                String::new(),
                Presence::Unknown,
                false
            )
            .is_ok(),
            accepted
        );
    }
}
