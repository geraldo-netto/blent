//! Linux cached descriptors only: no USB opens, claims or control transfers.
//! An admitted Task owns the entire read batch through native completion and join.
//! Each read admits at most 258 bytes; names live only in the bounded snapshot.
use super::Candidate;
use crate::owned_task::Task;
use anyhow::{Context, Result};
use blent_config::connection::control::{
    names::{unsafe_character, MAX_NAME_BYTES},
    DeviceNames,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use tokio::sync::oneshot;

pub(super) async fn enrich(candidates: Vec<Candidate>) -> Result<Vec<Candidate>> {
    let (worker, result) = start(PathBuf::from("/sys/bus/usb/devices"), candidates)?;
    worker.await??;
    result
        .await
        .context("USB descriptor worker returned no inventory")
}

fn start(
    root: PathBuf,
    mut candidates: Vec<Candidate>,
) -> Result<(Task, oneshot::Receiver<Vec<Candidate>>)> {
    let (output, result) = oneshot::channel();
    let worker = Task::spawn_blocking(move || {
        for candidate in &mut candidates {
            candidate.names = read(&root, candidate);
        }
        // Caller loss drops the owned result here; no names outlive their snapshot.
        let _ = output.send(candidates);
        Ok(())
    })?;
    Ok((worker, result))
}

fn read(root: &Path, candidate: &Candidate) -> DeviceNames {
    let (bus, ports) = candidate.physical_path();
    let ports = ports
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(".");
    let path = root.join(format!("{bus}-{ports}"));
    if !matches_candidate(&path, candidate) {
        return DeviceNames::default();
    }
    let manufacturer = attribute(&path.join("manufacturer")).unwrap_or_default();
    let model = attribute(&path.join("product")).unwrap_or_default();
    if !matches_candidate(&path, candidate) {
        return DeviceNames::default();
    }
    // Observations can race unplug/reuse. They are hints, never selection authority.
    DeviceNames::new(manufacturer, model).unwrap_or_default()
}

fn matches_candidate(path: &Path, candidate: &Candidate) -> bool {
    for (file, expected) in [
        ("idVendor", format!("{:04x}", candidate.vendor)),
        ("idProduct", format!("{:04x}", candidate.product)),
        ("devnum", candidate.address.to_string()),
    ] {
        if attribute(&path.join(file)).as_deref() != Some(expected.as_str()) {
            return false;
        }
    }
    true
}

fn attribute(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take((MAX_NAME_BYTES + 2) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_NAME_BYTES + 1 {
        return None;
    }
    let value = std::str::from_utf8(&bytes).ok()?.trim();
    if value.len() > MAX_NAME_BYTES || value.chars().any(unsafe_character) {
        return None;
    }
    Some(value.to_owned())
}

#[cfg(test)]
mod tests;
