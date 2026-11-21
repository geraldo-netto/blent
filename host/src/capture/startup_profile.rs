//! T1674: opt-in native encoder-start attribution; synthetic input only.
//! One original child, one bounded NV12 frame and one packet reader per sample.
//! No display, EVDI, camera, ADB, saved preference or installed process mutation.
use super::*;
use serde_json::json;
use tokio::io::{AsyncWriteExt, BufReader};

fn command(encoder: &str, size: (u32, u32), depth: bool) -> Result<Command> {
    let config = CaptureConfig {
        encoder: encoder.into(),
        width: size.0,
        height: size.1,
        fps: 60,
        instance: u32::MAX,
        ..Default::default()
    };
    let built = CliEncoder { config: &config }.encoder_command(size.0, size.1, depth)?;
    let mut args = built
        .as_std()
        .get_args()
        .map(std::ffi::OsStr::to_owned)
        .collect::<Vec<_>>();
    let input = args.iter().position(|arg| arg == "-i").unwrap();
    args[input + 1] = "pipe:0".into();
    let mut command = Command::new("ffmpeg");
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    Ok(command)
}

fn picture(size: (u32, u32)) -> Vec<u8> {
    let pixels = (size.0 * size.1) as usize;
    let mut frame = vec![128; pixels * 3 / 2];
    for (index, value) in frame[..pixels].iter_mut().enumerate() {
        *value = (16 + (((index / size.0 as usize / 8) ^ (index % size.0 as usize / 8)) * 37 % 220))
            as u8;
    }
    frame
}

async fn first_packet(encoder: &str, output: tokio::process::ChildStdout) -> Result<usize> {
    let mut input = BufReader::new(output);
    let mut parser = Packetizer::new(Codec::from_encoder(encoder), Default::default());
    loop {
        let (read, packets) = parser.read_from(&mut input).await?;
        anyhow::ensure!(read != 0, "encoder ended before first packet");
        if let Some(packet) = packets.first() {
            anyhow::ensure!(
                packets.len() == 1 && packet.is_idr,
                "unexpected initial packet"
            );
            return Ok(packet.data.len());
        }
    }
}

async fn observe(child: &mut tokio::process::Child, encoder: &str, frame: &[u8]) -> Result<usize> {
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(frame).await?;
    // Keep stdin open until the complete first packet arrives, with no successor.
    let packet = first_packet(encoder, child.stdout.take().unwrap()).await;
    drop(stdin);
    packet
}

async fn sample(encoder: &str, size: (u32, u32), repeat: usize) -> Result<serde_json::Value> {
    let frame = picture(size);
    let scope = blent::native_scope::Scope::default();
    let started = Instant::now();
    let depth = supports_async_depth(std::ffi::OsStr::new("ffmpeg"), encoder, &scope).await;
    let probe_us = started.elapsed().as_micros() as u64;
    let mut child = command(encoder, size, depth)?.spawn()?;
    // This is a no-successor protocol bound, not a total experiment deadline.
    let observed = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        observe(&mut child, encoder, &frame),
    )
    .await;
    let first_packet_us = started.elapsed().as_micros() as u64;
    let stop = child.kill().await;
    let status = child.wait().await?;
    stop?;
    let bytes = observed.context("first packet protocol deadline")??;
    Ok(
        json!({"encoder":encoder,"size":size,"repeat":repeat,"probe_us":probe_us,"first_packet_us":first_packet_us,"packet_bytes":bytes,"async_depth":depth,"original_child_waited":true,"exit":status.code()}),
    )
}

#[tokio::test]
#[ignore = "Opt-in T1674 native startup measurement; no installed session mutation"]
async fn t1674_native_startup_probe_attribution() {
    for repeat in 0..10 {
        for size in [(640, 400), (1280, 800), (1920, 1080)] {
            let encoders = if repeat % 2 == 0 {
                ["h264_vaapi", "hevc_vaapi"]
            } else {
                ["hevc_vaapi", "h264_vaapi"]
            };
            for encoder in encoders {
                let result = sample(encoder, size, repeat).await.unwrap();
                println!("T1674_SAMPLE {result}");
            }
        }
    }
}
