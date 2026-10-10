use super::*;
use serde_json::{json, Value};
fn record() -> Value {
    json!({"serial":"USB-tablet", "instance":0, "video_port":8890, "input_port":8891})
}
#[test]
fn t922_adb_wire_shape_is_preserved_and_native_identity_is_distinct() {
    let adb: TabletSession = serde_json::from_value(record()).unwrap();
    assert_eq!(adb.identity.adb_serial(), Some("USB-tablet"));
    assert!(adb.identity.peer().is_none());
    assert_eq!(adb.identity.label(), "USB-tablet");
    assert_eq!(serde_json::to_value(&adb).unwrap(), record());
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    let mut native = adb.clone();
    native.identity = Identity::UsbPeer(peer.clone());
    assert_eq!(native.identity.peer(), Some(&peer));
    assert!(native.identity.adb_serial().is_none());
    assert_eq!(native.identity.label(), "USB tablet (abababababab)");
    let wire = serde_json::to_value(&native).unwrap();
    assert!(wire.get("serial").is_none());
    assert_eq!(wire["peer"], peer.as_str());
    assert_eq!(
        serde_json::from_value::<TabletSession>(wire).unwrap(),
        native
    );
    let alias = Identity::Adb(peer.as_str().into());
    assert_ne!(alias, native.identity);
}
#[test]
fn t922_identity_schema_rejects_ambiguous_missing_and_invalid_inputs() {
    let mut both = record();
    both["peer"] = json!("a".repeat(64));
    let mut neither = record();
    neither.as_object_mut().unwrap().remove("serial");
    for invalid in [both, neither] {
        assert!(serde_json::from_value::<TabletSession>(invalid).is_err());
    }
    for byte in 0..=255u8 {
        if char::from(byte).is_control() {
            let mut value = record();
            value["serial"] = json!(format!("bad{}serial", char::from(byte)));
            assert!(serde_json::from_value::<TabletSession>(value).is_err());
        }
    }
    for invalid in [String::new(), "x".repeat(1025)] {
        let mut value = record();
        value["serial"] = json!(invalid);
        assert!(serde_json::from_value::<TabletSession>(value).is_err());
    }
    for invalid in ["", "abc", &"A".repeat(64), &"g".repeat(64), &"a".repeat(65)] {
        let mut value = record();
        value.as_object_mut().unwrap().remove("serial");
        value["peer"] = json!(invalid);
        assert!(serde_json::from_value::<TabletSession>(value).is_err());
    }
}
#[test]
fn t922_slot_and_port_boundaries_reject_on_read_and_write() {
    for (field, invalid) in [
        ("instance", 4i64),
        ("instance", -1),
        ("instance", i64::MAX),
        ("video_port", 0),
        ("video_port", 65536),
        ("input_port", 0),
        ("input_port", -1),
        ("input_port", 8890),
    ] {
        let mut value = record();
        value[field] = json!(invalid);
        assert!(serde_json::from_value::<TabletSession>(value).is_err());
    }
    let mut session: TabletSession = serde_json::from_value(record()).unwrap();
    session.instance = 3;
    session.video_port = u16::MAX;
    assert!(serde_json::to_value(&session).is_ok());
    session.identity = Identity::Adb("bad\nserial".into());
    assert!(serde_json::to_value(&session).is_err());
    session.identity = Identity::Adb("valid".into());
    session.instance = 4;
    assert!(serde_json::to_value(&session).is_err());
    session.instance = 0;
    session.input_port = session.video_port;
    assert!(serde_json::to_value(&session).is_err());
}
