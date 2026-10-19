//! T1107: qualified installed CPU recipes; no capture, device or application action.
use super::{Packetizer, Stdio};
use crate::media::{Codec, VideoPacket};
use anyhow::{ensure, Context, Result};
use blent::native_scope::Scope;
use blent_config::commands::OwnedChild;
use std::{
    ffi::OsStr,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
};

const WIDTH: usize = 64;
const PIXELS: usize = WIDTH * WIDTH;
const DELIVERY: Duration = Duration::from_secs(2);

struct Picture {
    packet: VideoPacket,
    timestamp: i64,
    delivered_us: u128,
}

struct Session {
    child: OwnedChild,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    parser: Packetizer,
    codec: Codec,
}

impl Session {
    fn start(encoder: &str, scope: &Scope) -> Result<Self> {
        let settings = blent::ffmpeg_args::Settings {
            encoder: encoder.into(),
            vaapi_device: String::new(),
            fps: 60,
            bitrate: 20000,
            quality: 18,
            workers: 1,
            adaptive_idle: false,
        };
        let arguments = settings.arguments(OsStr::new("pipe:0"), false, 64, 64)?;
        eprintln!("T1107 recipe {encoder}: {arguments:?}");
        let mut command = Command::new("ffmpeg");
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = scope.spawn(&mut command)?;
        let input = child.take_stdin().context("T1107 original encoder stdin")?;
        let output = BufReader::new(
            child
                .take_stdout()
                .context("T1107 original encoder stdout")?,
        );
        let codec = Codec::from_encoder(encoder);
        Ok(Self {
            child,
            input,
            output,
            parser: Packetizer::new(codec, Default::default()),
            codec,
        })
    }

    async fn picture(&mut self, frame: &[u8]) -> Result<Picture> {
        let started = Instant::now();
        tokio::time::timeout(DELIVERY, async {
            self.input.write_all(frame).await?;
            loop {
                let (read, mut packets) = self.parser.read_from(&mut self.output).await?;
                ensure!(read > 0, "T1107 premature encoder EOF");
                if packets.is_empty() {
                    continue;
                }
                ensure!(packets.len() == 1, "T1107 duplicated picture");
                return Ok(Picture {
                    packet: packets.remove(0),
                    timestamp: self
                        .parser
                        .timestamp_us()
                        .context("T1107 missing monotonic timestamp")?,
                    delivered_us: started.elapsed().as_micros(),
                });
            }
        })
        .await
        .context("T1107 picture requires a successor or EOF")?
    }

    fn still_open(&mut self) -> Result<()> {
        ensure!(
            self.child.try_wait()?.is_none(),
            "T1107 encoder exited with input still open"
        );
        Ok(())
    }
}

fn reference(marker: u8, cursor: bool) -> Vec<u8> {
    let mut frame = vec![128; PIXELS * 3 / 2];
    frame[..PIXELS].fill(marker);
    if cursor {
        for row in 24..40 {
            frame[row * WIDTH + 24..row * WIDTH + 40].fill(224);
        }
    }
    frame
}

async fn independent_decode(
    codec: Codec,
    picture: &Picture,
    reference: &[u8],
    scope: &Scope,
) -> Result<()> {
    ensure!(
        picture.packet.is_idr,
        "T1107 late join lacks independent keyframe"
    );
    ensure!(
        picture.packet.codec_config.is_some(),
        "T1107 missing current configuration"
    );
    let bytes = super::super::probe_format::sample(codec, 64, 64, &picture.packet);
    let mut command = Command::new("ffmpeg");
    command.args([
        "-v",
        "error",
        "-xerror",
        "-threads",
        "1",
        "-f",
        codec.muxer(),
        "-i",
        "pipe:0",
        "-frames:v",
        "1",
        "-pix_fmt",
        "nv12",
        "-f",
        "rawvideo",
        "pipe:1",
    ]);
    let decoded = scope.output(&mut command, Some(&bytes), DELIVERY).await?;
    ensure!(
        decoded.status.success(),
        "T1107 independent decode failed: {:?}",
        decoded.stderr
    );
    ensure!(
        decoded.stdout.len() == reference.len(),
        "T1107 decoded picture size mismatch"
    );
    for (&actual, &expected) in decoded.stdout.iter().zip(reference) {
        ensure!(
            actual.abs_diff(expected) <= 8,
            "T1107 picture identity differs: {actual} vs {expected}"
        );
    }
    Ok(())
}

fn continuity(previous: &Picture, current: &Picture) -> Result<()> {
    ensure!(
        previous.packet.seq.checked_add(1) == Some(current.packet.seq),
        "T1107 picture lost or repeated"
    );
    ensure!(
        current.timestamp > previous.timestamp,
        "T1107 timestamps not monotonic"
    );
    ensure!(
        std::sync::Arc::ptr_eq(&previous.packet.generation, &current.packet.generation),
        "T1107 changed encoder identity without replacement"
    );
    Ok(())
}

async fn resumed(
    session: &mut Session,
    before: &Picture,
    frame: &[u8],
    scope: &Scope,
) -> Result<Picture> {
    tokio::time::sleep(Duration::from_millis(1250)).await;
    let picture = session.picture(frame).await?;
    session.still_open()?;
    continuity(before, &picture)?;
    ensure!(
        picture.timestamp - before.timestamp >= 1_000_000,
        "T1107 static gap collapsed"
    );
    independent_decode(session.codec, &picture, frame, scope).await?;
    Ok(picture)
}

async fn observe(encoder: &str, scope: &Scope) -> Result<serde_json::Value> {
    let mut session = Session::start(encoder, scope)?;
    let first = session.picture(&reference(32, false)).await?;
    session.still_open()?;
    independent_decode(session.codec, &first, &reference(32, false), scope).await?;
    let idle = resumed(&mut session, &first, &reference(32, false), scope).await?;
    let motion = session.picture(&reference(96, false)).await?;
    continuity(&idle, &motion)?;
    let cursor = session.picture(&reference(96, true)).await?;
    continuity(&motion, &cursor)?;
    let late = resumed(&mut session, &cursor, &reference(96, true), scope).await?;
    let report = serde_json::json!({"encoder":encoder, "pictures":5,
        "first_us":first.delivered_us, "idle_resume_us":idle.delivered_us,
        "motion_us":motion.delivered_us, "cursor_us":cursor.delivered_us,
        "late_join_us":late.delivered_us, "static_gap_us":idle.timestamp-first.timestamp,
        "late_gap_us":late.timestamp-cursor.timestamp});
    session.still_open()?;
    Ok(report)
}

async fn qualify(encoder: &str) {
    let (scope, retirement) = Scope::default().group();
    let result = observe(encoder, &scope).await;
    drop(scope);
    // Dropped OwnedChild transfers original native ownership to its reaper.
    // The test retains observation until actual original cleanup, without a kill cap.
    while !retirement.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    eprintln!(
        "T1107 qualified with original native retirement: {}",
        result.unwrap()
    );
}

#[tokio::test]
async fn t1107_h264_sparse_input_remains_open_and_late_join_decodes() {
    qualify("libx264").await;
}

#[tokio::test]
async fn t1107_vp9_sparse_input_remains_open_and_late_join_decodes() {
    qualify("libvpx-vp9").await;
}

#[tokio::test]
async fn t1107_av1_sparse_input_remains_open_and_late_join_decodes() {
    qualify("libaom-av1").await;
}

fn control(sequence: u32, timestamp: i64) -> Picture {
    Picture {
        packet: VideoPacket {
            data: crate::media_storage::MediaBytes::from(vec![1]),
            is_idr: false,
            seq: sequence,
            codec_config: None,
            generation: Default::default(),
        },
        timestamp,
        delivered_us: 0,
    }
}

#[test]
fn t1107_qualification_rejects_duplicate_backward_and_foreign_output() {
    let first = control(0, 1_000_000);
    for sequence in [0, 2, u32::MAX] {
        let mut candidate = control(sequence, 2_000_000);
        candidate.packet.generation = first.packet.generation.clone();
        assert!(continuity(&first, &candidate).is_err());
    }
    for timestamp in [i64::MIN, -1, 0, 1_000_000] {
        let mut candidate = control(1, timestamp);
        candidate.packet.generation = first.packet.generation.clone();
        assert!(continuity(&first, &candidate).is_err());
    }
    assert!(continuity(&first, &control(1, 2_000_000)).is_err());
}

#[test]
fn t1107_exhausted_qualification_sequence_cannot_wrap_or_panic() {
    let first = control(u32::MAX, 0);
    let mut wrapped = control(0, 1);
    wrapped.packet.generation = first.packet.generation.clone();
    assert!(continuity(&first, &wrapped).is_err());
}

#[tokio::test]
async fn t1107_qualification_rejects_missing_key_or_configuration_before_native_decode() {
    let scope = Scope::default();
    let mut picture = control(0, 0);
    assert!(independent_decode(Codec::H264, &picture, &[], &scope)
        .await
        .is_err());
    picture.packet.is_idr = true;
    assert!(independent_decode(Codec::H264, &picture, &[], &scope)
        .await
        .is_err());
}
