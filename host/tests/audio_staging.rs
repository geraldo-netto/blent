//! T818: compare allocation-based staging with reusable production adapters.
#![cfg(target_os = "linux")]
#![allow(dead_code)]
#[path = "support/count_alloc.rs"]
mod counts;
#[path = "../src/audio/protocol.rs"]
mod protocol;
use blent_config::audio::*;
use std::{sync::atomic::Ordering, time::Instant};
use tokio::io::AsyncReadExt;

fn grant(direction: Direction) -> AudioGrant {
    AudioSession::new(direction)
        .start(
            AudioProfile::new(direction),
            AudioCapabilities {
                microphone: true,
                speakers: true,
                speech: true,
                raw: true,
                background: true,
            },
            true,
            0,
        )
        .unwrap()
}

// Original adapter's two allocations and PCM materialization, retained as oracle.
async fn legacy(input: &mut &[u8], frames: &mut FrameReader) -> anyhow::Result<Vec<u8>> {
    let mut header = [0; FRAME_HEADER_BYTES];
    input.read_exact(&mut header).await?;
    let size = frames.payload_bytes(&header)?;
    let mut packet = vec![0; header.len() + size];
    packet[..header.len()].copy_from_slice(&header);
    input.read_exact(&mut packet[header.len()..]).await?;
    let block = frames.decode(&packet)?;
    let mut native = Vec::with_capacity(1 + size);
    native.push(u8::from(block.discontinuity));
    native.extend_from_slice(&packet[header.len()..]);
    Ok(native)
}
fn start() {
    counts::ALLOCS.store(0, Ordering::Relaxed);
    counts::BYTES.store(0, Ordering::Relaxed);
    counts::ACTIVE.store(true, Ordering::Relaxed);
}
fn finish(
    mut times: Vec<u128>,
    count: usize,
    total: u128,
    reused: bool,
    direction: Direction,
    stage: &str,
) {
    counts::ACTIVE.store(false, Ordering::Relaxed);
    let allocations = counts::ALLOCS.load(Ordering::Relaxed);
    let bytes = counts::BYTES.load(Ordering::Relaxed);
    if reused {
        assert_eq!(allocations, 0, "T818: steady staging allocates");
    }
    times.sort_unstable();
    println!(
        "T818_STAGING {}",
        serde_json::json!({"stage":stage,"channels":direction.channels(),"reused":reused,"frames":count,"allocations":allocations,"allocated_bytes":bytes,"total_ns":total,"p50_ns":times[count/2],"p95_ns":times[count*95/100]})
    );
}
async fn receive(direction: Direction, count: usize, reused: bool) {
    let grant = grant(direction);
    let samples = vec![i16::MIN; BLOCK_FRAMES * direction.channels()];
    let packets: Vec<_> = (0..count)
        .map(|i| grant.encode(i as u64, i as u64 + 1, &samples).unwrap())
        .collect();
    let mut reader = grant.authenticate(&grant.hello()).unwrap();
    let mut staging = protocol::Staging::new();
    let mut times = Vec::with_capacity(count);
    start();
    let began = Instant::now();
    for packet in &packets {
        let time = Instant::now();
        if reused {
            let native = staging
                .packet(&mut packet.as_slice(), &mut reader)
                .await
                .unwrap();
            assert_eq!(&native[1..], &packet[28..]);
        } else {
            let native = legacy(&mut packet.as_slice(), &mut reader).await.unwrap();
            assert_eq!(&native[1..], &packet[28..]);
        }
        times.push(time.elapsed().as_nanos());
    }
    finish(
        times,
        count,
        began.elapsed().as_nanos(),
        reused,
        direction,
        "receive",
    );
}
fn send(direction: Direction, count: usize, reused: bool) {
    let grant = grant(direction);
    let mut writer = FrameWriter::new(grant);
    let block = PcmBlock::from_le_bytes(
        direction,
        &vec![91; BLOCK_FRAMES * direction.channels() * 2],
    )
    .unwrap();
    let mut bytes = Vec::with_capacity(FRAME_HEADER_BYTES + CLOCK_BYTES + MAX_SAMPLES * 2);
    let mut times = Vec::with_capacity(count);
    start();
    let began = Instant::now();
    for i in 0..count {
        let time = Instant::now();
        if reused {
            writer
                .encode_into(&block, i as u64 + 1, &mut bytes)
                .unwrap();
        } else {
            bytes = writer.encode(&block, i as u64 + 1).unwrap();
        }
        assert_eq!(
            bytes.len(),
            FRAME_HEADER_BYTES + BLOCK_FRAMES * direction.channels() * 2
        );
        assert_eq!(bytes[28], 91);
        std::hint::black_box(&bytes);
        times.push(time.elapsed().as_nanos());
    }
    finish(
        times,
        count,
        began.elapsed().as_nanos(),
        reused,
        direction,
        "send",
    );
}
#[tokio::test]
async fn t818_bounded_staging_has_zero_steady_allocations() {
    let measured = std::env::var_os("BLENT_T818_MEASURE").is_some();
    let count = if measured { 10000 } else { 64 };
    let pairs = if measured { 5 } else { 1 };
    for direction in [Direction::Microphone, Direction::Speakers] {
        for pair in 0..pairs {
            for reused in if pair % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                receive(direction, count, reused).await;
                send(direction, count, reused);
            }
        }
    }
}
