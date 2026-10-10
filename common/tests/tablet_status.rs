//! T922: native display status must not require an invented ADB serial.
#[test]
fn t922_native_assignment_is_representable_without_adb() {
    let record = serde_json::json!({"peer":"ab".repeat(32),"instance":0,"video_port":8890,"input_port":8891});
    assert!(
        serde_json::from_value::<blent_config::tablets::TabletSession>(record).is_ok(),
        "T922 native display identity requires an ADB alias"
    );
}
