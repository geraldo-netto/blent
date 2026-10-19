//! Host-owned webcam pipeline; independent of the display daemon/EVDI.
use crate::camera_control::{self as control, Report};
use blent_config::camera::CameraState as State;
pub(crate) mod bridge;
mod decoder;
mod frame_pool;
#[cfg(test)]
mod native_tests;
pub mod output_device;
mod outputs;
pub(crate) mod protocol;
#[cfg(test)]
mod retirement_tests;
#[cfg(test)]
mod status_tests;
#[cfg(test)]
mod tests;
#[cfg(feature = "accessory-transport")]
pub(crate) mod usb;

use anyhow::{Context, Result};
use blent_config::camera::{
    connection::{Connection, Target},
    CameraOptions, CameraProfile,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{path::PathBuf, sync::Arc};
type Producers = FuturesUnordered<crate::owned_task::Task>;
use tokio::{net::TcpListener, sync::watch};

/// Native implementation availability, not device/permission qualification.
pub const fn transport_capabilities() -> blent_config::camera::connection::Transports {
    blent_config::camera::connection::Transports {
        adb: true,
        usb: cfg!(feature = "accessory-transport"),
    }
}

pub async fn run(options: &CameraOptions) -> Result<()> {
    let (stop, stopped) = watch::channel(false);
    let (status, _) = watch::channel(State::Starting);
    let status = Report::new(status, watch::channel(None).0);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut work = std::pin::pin!(run_native(options.clone(), stopped, status));
    crate::control_outcome::with_signals(work.as_mut(), &stop, tokio::signal::ctrl_c(), async {
        terminate.recv().await;
    })
    .await
}

pub async fn run_controlled(
    profile: CameraProfile,
    stop: watch::Receiver<bool>,
    status: Report,
) -> Result<()> {
    run_native(
        CameraOptions {
            profile,
            ..Default::default()
        },
        stop,
        status,
    )
    .await
}

async fn run_native(
    options: CameraOptions,
    stop: watch::Receiver<bool>,
    status: Report,
) -> Result<()> {
    let result = run_attempt(options, stop, status.clone()).await;
    status.wait_native().await;
    result
}

async fn run_attempt(
    options: CameraOptions,
    mut stop: watch::Receiver<bool>,
    status: Report,
) -> Result<()> {
    options.validate()?;
    validate_transport(options.target()?)?;
    let devices = [
        Arc::new(outputs::open_device(&options.front_device, "Blent Front")?),
        Arc::new(outputs::open_device(&options.rear_device, "Blent Rear")?),
    ];
    let ffmpeg = executable("ffmpeg")?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let token = blent_config::runtime::random_token()?;
    let bridge = connection(&options.profile, &listener, &token, &status).await?;
    let session = Session {
        listener: &listener,
        invitation: bridge.as_ref(),
        token: &token,
        ffmpeg: &ffmpeg,
        options: &options,
    };
    let result = serve(session, &mut stop, &status, &devices).await;
    let cleanup = bridge.close().await;
    if let Err(error) = &cleanup {
        tracing::warn!(%error, "camera mapping cleanup failed");
    }
    drop(devices);
    result.and(cleanup)
}

fn validate_transport(target: Target<'_>) -> Result<()> {
    match target {
        Target::Adb(_) => Ok(()),
        #[cfg(feature = "accessory-transport")]
        Target::Usb(_) => Ok(()),
        #[cfg(not(feature = "accessory-transport"))]
        Target::Usb(_) => anyhow::bail!("direct-USB camera requires accessory-transport support"),
    }
}

async fn connection(
    profile: &CameraProfile,
    listener: &TcpListener,
    _token: &str,
    _status: &Report,
) -> Result<Box<dyn Connection<CameraProfile>>> {
    match profile.target()? {
        Target::Adb(serial) => Ok(Box::new(
            bridge::Bridge::create(executable("adb")?, serial, listener.local_addr()?.port())
                .await?,
        )),
        #[cfg(feature = "accessory-transport")]
        Target::Usb(_) => {
            use std::os::fd::AsFd;
            Ok(Box::new(usb::native(
                profile,
                _token,
                || {
                    Ok(std::net::TcpListener::from(
                        listener.as_fd().try_clone_to_owned()?,
                    ))
                },
                crate::broker::linux::client::Launch::from_environment()?,
                _status,
            )?))
        }
        #[cfg(not(feature = "accessory-transport"))]
        Target::Usb(_) => anyhow::bail!("direct-USB camera requires accessory-transport support"),
    }
}

fn executable(name: &str) -> Result<PathBuf> {
    blent_config::linux::programs::find_in(name, &std::env::var_os("PATH").unwrap_or_default())
        .with_context(|| format!("camera sharing requires {name}"))
}

async fn serve(
    session: Session<'_>,
    stop: &mut watch::Receiver<bool>,
    status: &Report,
    devices: &[Arc<std::fs::File>; 2],
) -> Result<()> {
    if *stop.borrow() {
        return Ok(());
    }
    let options = session.options;
    let ffmpeg = session.ffmpeg;
    let (front, front_rx) = watch::channel(None);
    let (rear, rear_rx) = watch::channel(None);
    let mut producers = Producers::new();
    let mut completions = Vec::with_capacity(2);
    for (index, (device, frames)) in [
        (&options.front_device, front_rx),
        (&options.rear_device, rear_rx),
    ]
    .into_iter()
    .enumerate()
    {
        let (work, completion) = producer_work(
            ffmpeg,
            device,
            options,
            frames,
            devices[index].clone(),
            status,
        )?;
        completions.push(completion);
        producers.push(work);
    }
    let frames = [front, rear];
    let result = session.invited(&frames, &mut producers, stop, status).await;
    drop(frames);
    if tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while producers.next().await.is_some() {}
    })
    .await
    .is_err()
    {
        producers.clear();
    }
    anyhow::ensure!(
        completions.iter().all(|ticket| ticket.is_complete()),
        "camera output retirement incomplete; device locks retained"
    );
    result
}

fn producer_work(
    ffmpeg: &std::path::Path,
    device: &std::path::Path,
    options: &CameraOptions,
    frames: watch::Receiver<Option<Arc<Vec<u8>>>>,
    lease: Arc<std::fs::File>,
    status: &Report,
) -> Result<(
    crate::owned_task::Task,
    blent_config::retirement::Completion,
)> {
    use crate::camera_control::NativeResource::Producer;
    let child_slot = status.reserve_native(Producer)?;
    let task_slot = status.reserve_native(Producer)?;
    let mut child =
        blent_config::commands::OwnedChild::spawn(&mut outputs::producer(ffmpeg, device, options))
            .context("start virtual camera producer")?;
    let completion = child.completion();
    child_slot.bind(completion.clone());
    child.keep_alive(lease);
    let feed_options = options.clone();
    let work = crate::owned_task::Task::spawn(async move {
        let result = outputs::feed(&mut child, frames, &feed_options).await;
        outputs::retire(child).await;
        result
    })?;
    task_slot.bind(work.completion());
    Ok((work, completion))
}

struct Session<'a> {
    listener: &'a TcpListener,
    invitation: &'a dyn Connection<CameraProfile>,
    token: &'a str,
    ffmpeg: &'a std::path::Path,
    options: &'a CameraOptions,
}

impl Session<'_> {
    async fn invited(
        &self,
        frames: &[outputs::Frames; 2],
        producers: &mut Producers,
        stop: &mut watch::Receiver<bool>,
        status: &Report,
    ) -> Result<()> {
        tokio::select! {
            result = self.invitation.invite(self.token, &self.options.profile) => result?,
            _ = control::cancelled(stop) => return Ok(()),
            result = producer_stopped(producers) => return result,
        }
        status.update(State::Waiting);
        tracing::info!("Webcams ready; waiting for tablet permission and camera frames.");
        tokio::select! {
            result = sessions(self.listener, self.token, self.ffmpeg, self.options, frames, status) => result,
            _ = control::cancelled(stop) => Ok(()),
            _ = frame_status(frames, self.options, status) => Ok(()),
            result = producer_stopped(producers) => result,
            result = self.invitation.monitor() => result,
        }
    }
}

async fn producer_stopped(producers: &mut Producers) -> Result<()> {
    producers
        .next()
        .await
        .context("camera producers stopped")???;
    anyhow::bail!("camera producer stopped")
}

async fn sessions(
    listener: &TcpListener,
    token: &str,
    ffmpeg: &std::path::Path,
    options: &CameraOptions,
    frames: &[outputs::Frames; 2],
    status: &Report,
) -> Result<()> {
    loop {
        let (socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;
        if let Err(error) =
            decoder::connection(socket, token, ffmpeg, options, frames, status.clone()).await
        {
            tracing::info!(%error, "camera connection ended; waiting for tablet selection");
        }
    }
}

async fn frame_status(frames: &[outputs::Frames; 2], options: &CameraOptions, status: &Report) {
    let mut selected = frames[match options.lens {
        blent_config::camera::Lens::Front => 0,
        blent_config::camera::Lens::Rear => 1,
    }]
    .subscribe();
    let mut next_preview = tokio::time::Instant::now();
    let mut expires = None;
    loop {
        tokio::select! {
            change = selected.changed() => if change.is_err() { return; },
            _ = async { tokio::time::sleep_until(expires.unwrap()).await }, if expires.is_some() => {
                expires = None;
                status.update(State::Waiting);
                continue;
            }
        }
        let frame = selected.borrow_and_update().clone();
        let Some(frame) = frame else {
            expires = None;
            status.update(State::Waiting);
            continue;
        };
        expires = Some(tokio::time::Instant::now() + outputs::FRESH_FRAME_AGE);
        status.update(State::Streaming);
        if tokio::time::Instant::now() >= next_preview {
            let preview = blent_config::camera::CameraPreview::from_yuv420(
                options.lens,
                options.width,
                options.height,
                &frame,
            )
            .ok()
            .map(std::sync::Arc::new);
            status.publish_preview(preview);
            next_preview = tokio::time::Instant::now() + std::time::Duration::from_millis(200);
        }
    }
}

#[cfg(test)]
mod connection_tests;
