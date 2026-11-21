use super::*;

mod package;

#[test]
fn t1545_unknown_state_is_present_but_not_ready() {
    for state in [
        "detached".to_owned(),
        "maintenance-mode_42".into(),
        "x".repeat(64),
    ] {
        let text = format!("List of devices attached\nREADY\tdevice\nOTHER\t{state}\n");
        assert_eq!(
            entries(&text),
            Some(vec![("READY", "device"), ("OTHER", state.as_str())]),
            "T1545"
        );
        assert_eq!(parse(&text), Some(vec!["READY".to_owned()]), "T1545");
    }
}

#[test]
fn t1545_state_syntax_and_capacity_still_reject_malformed_records() {
    for state in [
        "".to_owned(),
        "bad state".into(),
        "device\textra".into(),
        "dev\0ice".into(),
        "no permissions\0extra".into(),
        "x".repeat(65),
        format!("no permissions {}", "x".repeat(1024)),
    ] {
        assert_eq!(
            entries(&format!("List of devices attached\nUSB\t{state}\n")),
            None,
            "T1545: {state:?}"
        );
    }
}

#[test]
fn t1545_state_tokens_exhaust_single_byte_boundaries() {
    for code in 0..=255u32 {
        let state = format!("x{}y", char::from_u32(code).unwrap());
        let text = format!("List of devices attached\nUSB\t{state}\n");
        let allowed = matches!(code, 45 | 48..=57 | 95 | 97..=122);
        assert_eq!(
            entries(&text).is_some(),
            allowed,
            "T1545: state byte {code}"
        );
    }
    let diagnostic = format!("no permissions {}", "x".repeat(1009));
    assert_eq!(diagnostic.len(), 1024);
    assert!(entries(&format!("List of devices attached\nUSB\t{diagnostic}\n")).is_some());
}

#[test]
fn t1386_serial_contract_rejects_controls_and_oversize() {
    for code in (0..=31).chain(127..=159) {
        let serial = format!("a{}b", char::from_u32(code).unwrap());
        assert_eq!(
            parse(&format!("List of devices attached\n{serial}\tdevice\n")),
            None,
            "T1386: {code}"
        );
    }
    for size in [1025, 1026, 2048] {
        assert_eq!(
            parse(&format!(
                "List of devices attached\n{}\toffline\n",
                "x".repeat(size)
            )),
            None
        );
    }
    for serial in [
        "é".repeat(512),
        "x".repeat(1024),
        "tablet.local:5555".into(),
        "tablet._adb-tls-connect._tcp".into(),
    ] {
        assert_eq!(
            parse(&format!("List of devices attached\n{serial}\tdevice\n")),
            Some(vec![serial])
        );
    }
}

#[test]
fn t650_native_argument_nul_never_confirms_inventory() {
    for position in 0..=64 {
        let mut serial = "x".repeat(64);
        serial.insert(position, '\0');
        let inventory = format!("List of devices attached\n{serial}\tdevice\n");
        assert_eq!(
            parse(&inventory),
            None,
            "T650: NUL at serial offset {position}"
        );
    }
    assert_eq!(
        parse("List of devices attached\nUSB-café\tdevice\n"),
        Some(vec!["USB-café".into()])
    );
}

#[test]
fn t443_synthetic_devices_work_without_inventing_real_disconnects() {
    let previous = vec!["REAL".into()];
    assert_eq!(with_synthetic(None, &previous, vec![]), None);
    assert_eq!(
        with_synthetic(None, &previous, vec!["fake1".into()]),
        Some(vec!["REAL".into(), "fake1".into()])
    );
    assert_eq!(
        with_synthetic(Some(vec![]), &previous, vec!["fake1".into()]),
        Some(vec!["fake1".into()])
    );
}

#[test]
fn t443_inventory_parser_distinguishes_empty_unknown_and_nonready() {
    assert_eq!(parse("List of devices attached\n\n"), Some(vec![]));
    assert_eq!(parse("List of devices attached\nnet:5555\tdevice\nUSB\tdevice\nOFF\toffline\nNO\tno permissions (udev rules)\n"),
               Some(vec!["USB".into(), "net:5555".into()]));
    for malformed in [
        "",
        "error",
        "List of devices attached\npartial",
        "List of devices attached\nUSB\tbad state",
        "List of devices attached\nUSB\tdevice\nUSB\toffline",
    ] {
        assert_eq!(parse(malformed), None, "T443: {malformed}");
    }
}
