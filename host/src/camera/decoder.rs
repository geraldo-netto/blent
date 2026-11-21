use super::{outputs::Frames, protocol};
use anyhow::{Context, Result};
use blent_config::camera::CameraProfile;
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream, process::Command};

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod latency_tests;
#[cfg(test)]
mod retirement_tests;

pub fn filters(options: &CameraProfile) -> String {
    format!("scale={}:{}:force_original_aspect_ratio=decrease,colorspace=all=smpte170m:trc=bt709:range=tv:format=yuv420p,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=yuv420p",
        options.width, options.height, options.width, options.height)
}

fn command(ffmpeg: &Path, options: &CameraProfile) -> Result<Command> {
    // FFmpeg also checks aligned decoder buffers against max_pixels. Allow
    // SIMD/macroblock padding while keeping the decode allocation bounded.
    let capture = options.invitation_profile()?;
    let pixels =
        u32::from(capture.width).div_ceil(64) * 64 * u32::from(capture.height).div_ceil(64) * 64;
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
            &filters(options),
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
    Ok(command)
}

pub async fn connection(
    mut socket: TcpStream,
    token: &str,
    ffmpeg: &Path,
    options: &CameraProfile,
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
    let live = status
        .live
        .clone()
        .unwrap_or_else(|| crate::camera_live::Session::new(options.clone()));
    live.capabilities(header.capabilities);
    let mut command = command(ffmpeg, options)?;
    let original = status.reserve_native(crate::camera_control::NativeResource::Decoder)?;
    let mut decoder =
        blent_config::commands::OwnedChild::spawn(&mut command).context("start camera decoder")?;
    original.bind(decoder.completion());
    let result = transfer(
        &mut socket,
        &mut decoder,
        options,
        &outputs[header.lens],
        live,
        header.rotation,
    )
    .await;
    outputs[header.lens].send_replace(None);
    let retired = retire(decoder).await;
    anyhow::ensure!(
        retired.completion.is_complete(),
        "Camera decoder retirement incomplete; capacity retained"
    );
    result
}

async fn transfer(
    socket: &mut TcpStream,
    decoder: &mut tokio::process::Child,
    options: &CameraProfile,
    frames: &Frames,
    live: std::sync::Arc<crate::camera_live::Session>,
    rotation: u8,
) -> Result<()> {
    let mut input = decoder
        .stdin
        .take()
        .context("camera decoder stdin missing")?;
    let mut output = decoder
        .stdout
        .take()
        .context("camera decoder stdout missing")?;
    let mut pool = super::frame_pool::Pool::for_profile(options)?;
    pool.orient(rotation)?;
    pool.attach(live.clone());
    socket.write_all(b"OK").await?;
    let receive = receive_packets(
        socket,
        &mut input,
        Duration::from_millis(options.freshness_ms.into()),
        &live,
    );
    let decode = decode_frames(&mut pool, &mut output, frames);
    tokio::select! { result = receive => result, result = decode => result }
}

async fn receive_packets(
    socket: &mut TcpStream,
    input: &mut tokio::process::ChildStdin,
    budget: Duration,
    live: &crate::camera_live::Session,
) -> Result<()> {
    let mut sequence = 0u64;
    loop {
        let packet = tokio::time::timeout(
            Duration::from_secs(5),
            protocol::received_packet(socket, live),
        )
        .await??;
        let packet = super::source_color::normalize(&packet)?;
        tokio::time::timeout(budget, write_access_unit(input, &packet))
            .await
            .context("camera decoder input stalled")??;
        sequence = sequence
            .checked_add(1)
            .context("camera sequence exhausted")?;
        let feedback = [sequence.to_be_bytes().as_slice(), &live.command()?.encode()].concat();
        tokio::time::timeout(budget, socket.write_all(&feedback))
            .await
            .context("camera feedback stalled")??;
    }
}

/// Borrow one normalized packet (<= MAX_PACKET); the fixed six-byte delimiter
/// closes its picture for the raw H.264 parser without waiting for a successor.
/// Configuration-only packets do not introduce empty access units.
async fn write_access_unit(
    input: &mut (impl tokio::io::AsyncWrite + Unpin),
    packet: &[u8],
) -> std::io::Result<()> {
    input.write_all(packet).await?;
    let picture = crate::annex_scan::annex_b_offsets(packet).any(|(_, offset)| {
        packet
            .get(offset)
            .is_some_and(|header| matches!(header & 0x9f, 1..=5))
    });
    if picture {
        // AUD with primary_pic_type=7 (all slice types), then rbsp_stop_one_bit.
        input.write_all(&[0, 0, 0, 1, 9, 0xf0]).await?;
    }
    Ok(())
}

async fn decode_frames(
    pool: &mut super::frame_pool::Pool,
    output: &mut (impl tokio::io::AsyncRead + Unpin),
    frames: &Frames,
) -> Result<()> {
    loop {
        if let Some(bytes) = pool.read(output).await? {
            frames.send_replace(Some(super::outputs::Frame::new(bytes)));
        }
    }
}

pub(super) async fn retire(
    child: blent_config::commands::OwnedChild,
) -> blent_config::commands::Retirement {
    let retired = child.retire(Duration::from_secs(1)).await;
    if !retired.completion.is_complete() {
        tracing::warn!("Camera process retirement incomplete; capacity retained");
    }
    retired
}
