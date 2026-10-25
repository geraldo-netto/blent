//! Opt-in native benchmark: real tablet → production decoder, no V4L2 required.
//! Outputs timing/counts only; never stores camera pictures. Requests scoped capture stop after measurement.
#![allow(dead_code)]
#[path = "../../src/camera/bridge.rs"]
mod bridge;
#[path = "../../src/camera/decoder.rs"]
mod decoder;
// T816: compile the production decoder with its bounded lease owner.
#[path = "../../src/camera/frame_pool.rs"]
mod frame_pool;
#[path = "options.rs"]
mod options;
#[path = "../../src/camera/outputs.rs"]
mod outputs;
#[path = "ownership.rs"]
mod ownership;
#[path = "policy.rs"]
mod policy;
#[path = "../../src/camera/protocol.rs"]
mod protocol;
#[path = "samples.rs"]
mod samples;
mod stop;
use options::Options;
use stop::stop_capture;

use anyhow::{Context, Result};
use blent::camera_control::{self as control, Report};
use blent_config::camera::CameraProfile;
use clap::Parser;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch, time::Instant};

#[tokio::main]
pub(super) async fn run() -> Result<()> {
    let args = Options::parse();
    policy::validate(&args.camera, args.seconds, args.stall_feedback_ms)?;
    let samples = samples::Samples::new(args.max_samples)?;
    let listener = Arc::new(TcpListener::bind("127.0.0.1:0").await?);
    let target = listener.local_addr()?;
    let route = TcpListener::bind("127.0.0.1:0").await?;
    let token = blent_config::runtime::random_token()?;
    let (created_mapping, mapping_retirement) = ownership::channel();
    let mapping_creation = Mutex::new(Some(created_mapping));
    let created = bridge::Bridge::create(
        "/usr/bin/adb".into(),
        args.camera.serial.as_deref(),
        &route,
        &|proof| {
            mapping_creation.lock().unwrap().take().unwrap().bind(proof);
            Ok(())
        },
    )
    .await;
    drop(mapping_creation);
    let bridge = match created {
        Ok(bridge) => bridge,
        Err(error) => {
            mapping_retirement.wait().await;
            return Err(error);
        }
    };
    let (creation, retirement) = ownership::channel();
    let result = {
        tokio::select! {
            result = sample(listener.clone(), &bridge, &token, &args, samples, creation) => result,
            result = proxy(&route, target, args.stall_feedback_ms) => result,
        }
    };
    let stop = stop_capture(bridge.target(), &token).await;
    retirement.wait().await;
    let cleanup = bridge.close().await;
    mapping_retirement.wait().await;
    result?;
    stop?;
    cleanup
}

async fn sample(
    listener: Arc<TcpListener>,
    bridge: &bridge::Bridge,
    token: &str,
    args: &Options,
    mut samples: samples::Samples,
    creation: ownership::Creation,
) -> Result<()> {
    let (front, front_rx) = watch::channel(None);
    let (rear, rear_rx) = watch::channel(None);
    let frames: [outputs::Frames; 2] = [front, rear];
    let received = match args.camera.lens {
        blent_config::camera::Lens::Front => front_rx,
        blent_config::camera::Lens::Rear => rear_rx,
    };
    let owned_token = token.to_owned();
    let _owner = creation.start(args.camera.clone(), move |profile, stop, report| {
        decode_session(listener, owned_token, profile, frames, stop, report)
    })?;
    bridge.invite(token, &args.camera).await?;
    let started = Instant::now();
    tokio::select! {
        result = observe(received, &mut samples, started) => result?,
        _ = tokio::time::sleep(Duration::from_secs(args.seconds)) => {},
    }
    print_samples(args, &samples);
    anyhow::ensure!(samples.count() > 0, "no native camera frames decoded");
    Ok(())
}

async fn decode_session(
    listener: Arc<TcpListener>,
    token: String,
    profile: CameraProfile,
    frames: [outputs::Frames; 2],
    mut stop: watch::Receiver<bool>,
    report: Report,
) -> Result<()> {
    tokio::select! {
        result = connections(&listener, &token, &profile, &frames, report) => result,
        _ = control::cancelled(&mut stop) => Ok(()),
    }
}

async fn connections(
    listener: &TcpListener,
    token: &str,
    profile: &CameraProfile,
    frames: &[outputs::Frames; 2],
    report: Report,
) -> Result<()> {
    loop {
        let (socket, _) = listener.accept().await?;
        let result = decoder::connection(
            socket,
            token,
            Path::new("/usr/bin/ffmpeg"),
            profile,
            frames,
            report.clone(),
        )
        .await;
        eprintln!("camera generation ended: {result:?}");
    }
    #[allow(unreachable_code)]
    Ok(())
}

async fn observe(
    mut received: watch::Receiver<Option<Arc<Vec<u8>>>>,
    samples: &mut samples::Samples,
    started: Instant,
) -> Result<()> {
    loop {
        received.changed().await.context("probe output closed")?;
        if received.borrow_and_update().is_some() {
            samples.record(started.elapsed())?;
        }
    }
    #[allow(unreachable_code)]
    Ok(())
}

fn print_samples(args: &Options, samples: &samples::Samples) {
    println!(
        "{}",
        serde_json::json!({"seconds": args.seconds, "frames": samples.count(), "decoded_ms": samples.timestamps(), "stall_feedback_ms": args.stall_feedback_ms, "first_frame_ms": samples.first(),
        "bitrate_kbps": args.camera.bitrate, "adaptive": args.camera.adaptive_bitrate,
        "freshness_ms": args.camera.freshness_ms, "boundary": "camera sensor through production decoder; no V4L2/presentation"})
    );
}

async fn proxy(listener: &TcpListener, target: std::net::SocketAddr, stall_ms: u64) -> Result<()> {
    let started = Instant::now();
    let mut stalled = stall_ms == 0;
    loop {
        let (mut tablet, _) = listener.accept().await?;
        let mut host = tokio::net::TcpStream::connect(target).await?;
        tablet.set_nodelay(true)?;
        host.set_nodelay(true)?;
        let (mut tablet_in, mut tablet_out) = tablet.split();
        let (mut host_in, mut host_out) = host.split();
        tokio::select! {
            _ = tokio::io::copy(&mut tablet_in, &mut host_out) => {},
            _ = feedback(&mut host_in, &mut tablet_out, &mut stalled, started, stall_ms) => {},
        }
    }
}

async fn feedback(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
    writer: &mut (impl tokio::io::AsyncWrite + Unpin),
    stalled: &mut bool,
    started: Instant,
    stall_ms: u64,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut bytes = [0; 8];
    loop {
        let size = reader.read(&mut bytes).await?;
        if size == 0 {
            return Ok(());
        }
        if !*stalled && started.elapsed() >= Duration::from_secs(2) {
            *stalled = true;
            eprintln!(
                "inject feedback stall at {} ms",
                started.elapsed().as_millis()
            );
            tokio::time::sleep(Duration::from_millis(stall_ms)).await;
        }
        writer.write_all(&bytes[..size]).await?;
    }
}
