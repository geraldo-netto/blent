//! Original-file output workflow. One admitted blocking worker owns all ioctls
//! and writes; its async controller only owns cancellation and the actual join.
use super::{
    output_device::{self, ConfiguredOutput, Spec},
    output_writer::Writer,
    outputs,
};
use crate::{
    camera_control::{NativeResource, Report},
    owned_task::Task,
};
use anyhow::Result;
use blent_config::{camera::CameraProfile, retirement::Completion};
use std::{fs::File, sync::Arc, time::Duration};
use tokio::{
    sync::watch,
    time::{Instant, Interval},
};

type Configure = fn(File, Spec) -> Result<ConfiguredOutput>;
type Frames = watch::Receiver<Option<Arc<Vec<u8>>>>;

/// Admit both observations before effects. Dropping the returned controller
/// closes the worker's stop authority; Completion fences actual native return,
/// owned-file destruction and join, including failed/cancelled startup.
pub fn start(
    lease: Arc<File>,
    profile: &CameraProfile,
    frames: Frames,
    report: &Report,
) -> Result<(Task, Completion)> {
    start_with(
        lease,
        profile,
        frames,
        report,
        output_device::configure_blocking,
    )
}

fn start_with(
    lease: Arc<File>,
    profile: &CameraProfile,
    frames: Frames,
    report: &Report,
    configure: Configure,
) -> Result<(Task, Completion)> {
    let spec = Spec::from_profile(profile)?;
    let native_slot = report.reserve_native(NativeResource::Producer)?;
    let controller_slot = report.reserve_native(NativeResource::Producer)?;
    let black = outputs::blank(profile)?;
    let (running, stopped) = watch::channel(false);
    let job = Job {
        lease,
        spec,
        black,
        frames,
        stopped,
        configure,
    };
    let native = Task::spawn_blocking(move || job.run())?;
    let completion = native.completion();
    native_slot.bind(completion.clone());
    let controller = Task::spawn(async move {
        // Retained even before the first poll; Drop also handles rejected admission.
        let cancellation = running;
        let result = native.await;
        drop(cancellation);
        result?
    })?;
    controller_slot.bind(controller.completion());
    Ok((controller, completion))
}

struct Job {
    lease: Arc<File>,
    spec: Spec,
    black: Vec<u8>,
    frames: Frames,
    stopped: watch::Receiver<bool>,
    configure: Configure,
}
impl Job {
    fn run(self) -> Result<()> {
        if self.stopped.has_changed().is_err() {
            return Ok(());
        }
        let configured = (self.configure)(self.lease.try_clone()?, self.spec)?;
        if self.stopped.has_changed().is_err() {
            return Ok(());
        }
        tokio::runtime::Handle::current().block_on(feed(
            Writer::new(configured),
            self.frames,
            self.stopped,
            self.black,
        ))
    }
}

enum Event {
    Frame,
    Tick,
    Stop,
}
async fn next(frames: &mut Frames, stop: &mut watch::Receiver<bool>, tick: &mut Interval) -> Event {
    tokio::select! {
        biased;
        _ = crate::camera_control::cancelled(stop) => Event::Stop,
        change = frames.changed() => if change.is_ok() { Event::Frame } else { Event::Stop },
        _ = tick.tick() => Event::Tick,
    }
}
async fn feed(
    mut writer: Writer,
    mut frames: Frames,
    mut stop: watch::Receiver<bool>,
    black: Vec<u8>,
) -> Result<()> {
    let mut last_frame = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        match next(&mut frames, &mut stop, &mut tick).await {
            Event::Stop => return Ok(writer.write(&black)?),
            Event::Frame => last_frame = Instant::now(),
            Event::Tick => {}
        }
        let newest = frames.borrow_and_update().clone();
        let bytes = outputs::current_frame(newest.as_deref(), &black, last_frame.elapsed());
        writer.write(bytes)?;
    }
}

#[cfg(test)]
mod tests;
