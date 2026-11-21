//! T443: only a successful, well-formed ADB listing confirms disappearance.
use blent_config::adb::{transport_of, Transport};
use blent_config::commands::AsyncCommandExt;

pub async fn query(adb: &str) -> Option<Vec<String>> {
    let output = tokio::process::Command::new(adb)
        .arg("devices")
        .output_bounded()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse(std::str::from_utf8(&output.stdout).ok()?)
}

pub fn with_synthetic(
    confirmed: Option<Vec<String>>,
    previous: &[String],
    synthetic: Vec<String>,
) -> Option<Vec<String>> {
    let mut devices = confirmed.or_else(|| (!synthetic.is_empty()).then(|| previous.to_vec()))?;
    for serial in synthetic {
        if !devices.contains(&serial) {
            devices.push(serial);
        }
    }
    Some(devices)
}

fn valid_state(state: &str) -> bool {
    // ADB's short listing uses a tab before the complete connection state.
    // Missing udev access includes a diagnostic after "no permissions".
    if state == "no permissions" || state.starts_with("no permissions ") {
        return state.len() <= 1024 && !state.chars().any(char::is_control);
    }
    state.len() <= 64
        && state
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && state.bytes().all(state_token_byte)
}

fn state_token_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
}

fn record(line: &str) -> Option<(&str, &str)> {
    let (serial, state) = line.split_once('\t')?;
    blent_config::tablets::validate_adb_serial(serial).ok()?;
    if !valid_state(state) {
        return None;
    }
    Some((serial, state))
}

pub fn parse(text: &str) -> Option<Vec<String>> {
    let mut ready: Vec<_> = entries(text)?
        .into_iter()
        .filter(|(_, state)| *state == "device")
        .map(|(serial, _)| serial.to_owned())
        .collect();
    ready.sort_by_key(|serial| transport_of(serial) != Transport::Usb);
    Some(ready)
}

pub(crate) fn entries(text: &str) -> Option<Vec<(&str, &str)>> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    if lines.next()? != "List of devices attached" {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let mut records = Vec::new();
    for line in lines {
        let (serial, state) = record(line)?;
        if !seen.insert(serial) {
            return None;
        }
        records.push((serial, state));
    }
    Some(records)
}

pub fn package_presence(out: &std::process::Output) -> Option<bool> {
    if !out.stderr.is_empty() {
        return None;
    }
    let text = std::str::from_utf8(&out.stdout).ok()?.trim();
    // Current PackageManagerShellCommand reports absence with exit 1 and no paths.
    if text.is_empty() {
        return (out.status.code() == Some(1)).then_some(false);
    }
    if out.status.success() && text.lines().all(|line| line.starts_with("package:/")) {
        return Some(true);
    }
    None
}

#[cfg(test)]
mod tests {
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
}
