//! Linux output admission and original file retirement stay off the coordinator.
use super::{control, outputs, Report};
use crate::owned_task::Task;
use anyhow::{Context, Result};
use blent_config::{camera::CameraOptions, retirement::Admission, retirement_work::Cleanup};
use std::{fs::File, path::PathBuf, sync::Arc};
use tokio::sync::{oneshot, watch};

type Files = [Arc<File>; 2];
type Reply = oneshot::Sender<Result<Option<Devices>>>;

/// The native constructor admits its close sponsor before opening any file.
/// Borrowed views may only be handed to separately observed original workers.
pub(super) struct Devices(Option<Lease>);
struct Lease {
    files: Files,
    admission: Admission,
}
impl Devices {
    pub(super) fn files(&self) -> &Files {
        &self.0.as_ref().expect("live output lease").files
    }
}
impl Drop for Devices {
    fn drop(&mut self) {
        let Lease { files, admission } = self.0.take().expect("live output lease");
        admission.retain(Cleanup::new(move || drop(files)));
    }
}

pub(super) async fn admit(
    options: &CameraOptions,
    stop: &mut watch::Receiver<bool>,
    report: &Report,
) -> Result<Option<Devices>> {
    if stopped(stop) {
        return Ok(None);
    }
    let (work, reply) = start(options, stop.clone(), report)?;
    let result = tokio::select! { biased;
        _ = control::cancelled(stop) => Ok(None),
        result = reply => result.context("camera output constructor lost its reply")?,
    };
    drop(work); // Actual join remains independently observed, including cancellation.
    let devices = result?;
    if stopped(stop) {
        return Ok(None);
    }
    Ok(devices)
}
fn start(
    options: &CameraOptions,
    stop: watch::Receiver<bool>,
    report: &Report,
) -> Result<(Task, oneshot::Receiver<Result<Option<Devices>>>)> {
    let creator = report.reserve_native(control::NativeResource::Devices)?;
    let original = report.reserve_native(control::NativeResource::Devices)?;
    let admission = crate::work_admission::reserve()?;
    original.bind(admission.completion());
    let (ready, reply) = oneshot::channel();
    let paths = [options.front_device.clone(), options.rear_device.clone()];
    let work = Task::spawn_blocking(move || construct(paths, stop, ready, admission))?;
    creator.bind(work.completion());
    Ok((work, reply))
}
fn construct(
    paths: [PathBuf; 2],
    stop: watch::Receiver<bool>,
    ready: Reply,
    admission: Admission,
) -> Result<()> {
    let result = open_pair(&paths, &stop, &ready)
        .map(|files| files.map(|files| Devices(Some(Lease { files, admission }))));
    // A rejected/late result retires through its already admitted close sponsor.
    let _ = ready.send(result);
    Ok(())
}
fn open_pair(
    paths: &[PathBuf; 2],
    stop: &watch::Receiver<bool>,
    ready: &Reply,
) -> Result<Option<Files>> {
    if cancelled(stop, ready) {
        return Ok(None);
    }
    let front = Arc::new(outputs::open_device(&paths[0], "Blent Front")?);
    if cancelled(stop, ready) {
        return Ok(None);
    }
    let rear = Arc::new(outputs::open_device(&paths[1], "Blent Rear")?);
    if cancelled(stop, ready) {
        return Ok(None);
    }
    Ok(Some([front, rear]))
}
fn cancelled(stop: &watch::Receiver<bool>, ready: &Reply) -> bool {
    ready.is_closed() || stopped(stop)
}
pub(super) fn stopped(stop: &watch::Receiver<bool>) -> bool {
    *stop.borrow() || stop.has_changed().is_err()
}

#[cfg(test)]
mod tests;
