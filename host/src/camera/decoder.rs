use super::{
    outputs::{self, Frames},
    protocol,
};
use anyhow::{Context, Result};
use blent_config::camera::CameraOptions;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream, process::Command};

#[cfg(test)]
mod retirement_tests;

pub fn filters(rotation: u8, options: &CameraOptions) -> String {
    let rotation = (rotation + (options.rotation / 90) as u8) % 4;
    let turn = match rotation {
        1 => "transpose=clock,",
        2 => "hflip,vflip,",
        3 => "transpose=cclock,",
        _ => "",
    };
    let mirror = if options.mirror { "hflip," } else { "" };
    format!("{turn}{mirror}scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=yuv420p",
        options.width, options.height, options.width, options.height)
}

fn command(ffmpeg: &Path, rotation: u8, options: &CameraOptions) -> Command {
    // FFmpeg also checks aligned decoder buffers against max_pixels. Allow
    // SIMD/macroblock padding while keeping the decode allocation bounded.
    let pixels = options.width.div_ceil(64) * 64 * options.height.div_ceil(64) * 64;
    let mut command = Command::new(ffmpeg);
    command
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-max_alloc",
            "67108864",
            "-max_pixels",
            &pixels.to_string(),
            "-filter_threads",
            "1",
            "-threads",
            "1",
            "-flags",
            "low_delay",
            "-probesize",
            "32",
            "-analyzeduration",
            "0",
            "-f",
            "h264",
            "-i",
            "pipe:0",
            "-an",
            "-vf",
            &filters(rotation, options),
            "-threads",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    command
}

pub async fn connection(
    mut socket: TcpStream,
    token: &str,
    ffmpeg: &Path,
    options: &CameraOptions,
    outputs: &[Frames; 2],
    status: crate::camera_control::Report,
) -> Result<()> {
    let header = tokio::time::timeout(Duration::from_secs(5), protocol::header(&mut socket, token))
        .await??;
    let selected = match options.lens {
        blent_config::camera::Lens::Front => 0,
        blent_config::camera::Lens::Rear => 1,
    };
    anyhow::ensure!(
        header.lens == selected,
        "camera lens was not selected by host"
    );
    let original = status.reserve_native(crate::camera_control::NativeResource::Decoder)?;
    let mut decoder =
        blent_config::commands::OwnedChild::spawn(&mut command(ffmpeg, header.rotation, options))
            .context("start camera decoder")?;
    original.bind(decoder.completion());
    let result = transfer(&mut socket, &mut decoder, options, &outputs[header.lens]).await;
    outputs[header.lens].send_replace(None);
    let retired = outputs::retire(decoder).await;
    anyhow::ensure!(
        retired.completion.is_complete(),
        "Camera decoder retirement incomplete; capacity retained"
    );
    result
}

async fn transfer(
    socket: &mut TcpStream,
    decoder: &mut tokio::process::Child,
    options: &CameraOptions,
    frames: &Frames,
) -> Result<()> {
    let mut input = decoder
        .stdin
        .take()
        .context("camera decoder stdin missing")?;
    let mut output = decoder
        .stdout
        .take()
        .context("camera decoder stdout missing")?;
    let mut pool = super::frame_pool::Pool::configured(options.frame_bytes()?)?;
    socket.write_all(b"OK").await?;
    let receive = async {
        let mut sequence = 0u64;
        loop {
            let packet =
                tokio::time::timeout(Duration::from_secs(5), protocol::packet(socket)).await??;
            tokio::time::timeout(
                Duration::from_millis(options.freshness_ms.into()),
                input.write_all(&packet),
            )
            .await
            .context("camera decoder input stalled")??;
            sequence = sequence
                .checked_add(1)
                .context("camera sequence exhausted")?;
            tokio::time::timeout(
                Duration::from_millis(options.freshness_ms.into()),
                socket.write_u64(sequence),
            )
            .await
            .context("camera feedback stalled")??;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let decode = async {
        loop {
            if let Some(bytes) = pool.read(&mut output).await? {
                frames.send_replace(Some(bytes));
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::select! { result = receive => result, result = decode => result }
}
