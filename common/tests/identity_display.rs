//! T1537: externally supplied identity text follows one display-character contract.
use blent_config::{
    connection::routes::Identity as RouteIdentity,
    tablets::{validate_adb_serial, Identity},
    workspace::{Device, Presence},
};

const DIRECTION_CONTROLS: [char; 12] = [
    '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
];

fn device(name: &str) -> anyhow::Result<Device> {
    Device::new(
        Identity::Adb("tablet".into()),
        name.into(),
        Presence::Available,
        false,
    )
}

#[test]
fn t1537_adb_identities_reject_direction_controls() {
    for character in DIRECTION_CONTROLS {
        let value = format!("tablet{character}serial");
        assert!(validate_adb_serial(&value).is_err(), "{character:?}");
        assert!(Device::new(
            Identity::Adb(value),
            String::new(),
            Presence::Available,
            false
        )
        .is_err());
    }
}

#[test]
fn t1537_workspace_names_reject_direction_controls() {
    for character in DIRECTION_CONTROLS {
        let value = format!("Tablet{character}name");
        assert!(device(&value).is_err(), "{character:?}");
    }
}

#[test]
fn t1537_route_keys_and_profiles_reject_direction_controls() {
    for character in DIRECTION_CONTROLS {
        let value = format!("tablet{character}identity");
        assert!(RouteIdentity::observed(value.clone(), None).is_err());
        assert!(RouteIdentity::observed("valid".into(), Some(value)).is_err());
    }
}

#[test]
fn t1537_identity_limits_and_international_text_remain_supported() {
    for value in [
        "tablet",
        "平板",
        "تابلت",
        "טאבלט",
        "tablet._adb-tls-connect._tcp.local.",
    ] {
        assert!(validate_adb_serial(value).is_ok());
        assert!(device(value).is_ok());
        assert!(RouteIdentity::observed(value.into(), Some(value.into())).is_ok());
    }
    for length in [0, 1, 1023, 1024, 1025, 2048] {
        let value = "x".repeat(length);
        assert_eq!(
            validate_adb_serial(&value).is_ok(),
            (1..=1024).contains(&length)
        );
        assert_eq!(device(&value).is_ok(), length <= 1024);
        assert_eq!(
            RouteIdentity::observed(value, None).is_ok(),
            (1..=1024).contains(&length)
        );
    }
    assert_eq!(device("").unwrap().name(), "tablet");
}

#[test]
fn t1537_all_c0_and_c1_controls_are_rejected_at_text_boundaries() {
    for code in (0..=31).chain(127..=159) {
        let character = char::from_u32(code).unwrap();
        for value in [format!("{character}tablet"), format!("tablet{character}")] {
            assert!(validate_adb_serial(&value).is_err());
            assert!(device(&value).is_err());
            assert!(RouteIdentity::observed(value.clone(), Some(value)).is_err());
        }
    }
}
