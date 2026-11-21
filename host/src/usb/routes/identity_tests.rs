//! T1659: route admission and retargeting share the current identity contract.
use super::*;

const DIRECTION_CONTROLS: [char; 12] = [
    '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
];

#[test]
fn t1659_route_admission_rejects_direction_controls() {
    for character in DIRECTION_CONTROLS {
        let serial = format!("tablet{character}:5555");
        assert!(Routes::for_transport(&serial, (19000, 19001)).is_err());
    }
}

#[test]
fn t1659_rejected_retarget_preserves_original_serial_and_control() {
    let mut routes = Routes::for_transport("original:5555", (19000, 19001)).unwrap();
    let original = routes.serial.clone();
    for character in DIRECTION_CONTROLS {
        let serial = format!("tablet{character}:5555");
        assert!(routes.retarget(&serial).is_err());
        assert!(Arc::ptr_eq(&original, &routes.serial));
        assert_eq!(routes.serial(), "original:5555");
        assert_eq!(routes.expected, [(8890, 19000), (8891, 19001)]);
    }
}

#[test]
fn t1659_route_identity_boundaries_preserve_current_valid_forms() {
    let mut routes = Routes::for_transport("original", (19000, 19001)).unwrap();
    for length in [0, 1, 1023, 1024, 1025, 2048] {
        let serial = "x".repeat(length);
        let valid = (1..=1024).contains(&length);
        assert_eq!(
            Routes::for_transport(&serial, (19000, 19001)).is_ok(),
            valid
        );
        assert_eq!(routes.retarget(&serial).is_ok(), valid);
    }
    for serial in ["host:5555", "[2001:db8::1]:5555", "平板", "تابلت", "טאבלט"] {
        assert!(Routes::for_transport(serial, (19000, 19001)).is_ok());
        routes.retarget(serial).unwrap();
        assert_eq!(routes.serial(), serial);
    }
}
