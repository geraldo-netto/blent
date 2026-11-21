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
#[path = "adb_inventory/tests/mod.rs"]
mod tests;
