//! T785: capacity must not change framing, bounds or ready-packet publication.
use super::{Codec, Packetizer};
use tokio::io::{AsyncWriteExt, BufReader};
use std::time::Duration;

fn fixture(codec: Codec, size: usize, count: usize) -> Vec<Vec<u8>> {
    let mut payload = vec![0x55; size];
    if codec == Codec::H264 {
        payload[..19].copy_from_slice(&[0,0,0,1,0x67,0x11,0,0,0,1,0x68,0x22,0,0,0,1,0x65,0x80,0x33]);
        let packets = vec![payload; count];
        let stream = crate::framed_annex_b::tests::stream(codec, &packets);
        // Split the independently framed stream into complete producer writes.
        let prefix = stream.iter().enumerate().filter(|(_, b)| **b == b'\n').nth(3).unwrap().0 + 1;
        let mut offset = prefix;
        return (0..count).map(|index| {
            let line = stream[offset..].iter().position(|b| *b == b'\n').unwrap() + 1;
            let start = if index == 0 { 0 } else { offset };
            offset += line + size;
            stream[start..offset].to_vec()
        }).collect();
    }
    payload[..4].copy_from_slice(&[0x82, 0x49, 0x83, 0x42]);
    (0..count).map(|index| {
        let mut bytes = Vec::new();
        if index == 0 { bytes.extend_from_slice(b"DKIF\0\0\x20\0VP90\x40\0\x40\0\x1e\0\0\0\x01\0\0\0\0\0\0\0\0\0\0\0"); }
        bytes.extend_from_slice(&(size as u32).to_le_bytes());
        bytes.extend_from_slice(&(index as u64).to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes
    }).collect()
}

async fn ready_packet(codec: Codec, capacity: usize, size: usize) {
    let data = fixture(codec, size, 1).pop().unwrap();
    let (mut writer, reader) = tokio::io::duplex(data.len());
    writer.write_all(&data).await.unwrap();
    // Keep writer open and idle: returning requires no following byte or EOF.
    let mut reader = BufReader::with_capacity(capacity, reader);
    let mut parser = Packetizer::new(codec, Default::default());
    let (_, packets) = tokio::time::timeout(Duration::from_secs(1), parser.read_from(&mut reader)).await.unwrap().unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].seq, 0);
    assert!(packets[0].is_idr && packets[0].codec_config.is_some());
    assert_eq!(&packets[0].data[packets[0].data.len()-8..], &[0x55; 8]);
    let pending = tokio::time::timeout(Duration::from_millis(1), parser.read_from(&mut reader)).await;
    assert!(pending.is_err()); // cancellation drops a pending read, no spin.
    drop(writer);
}

#[tokio::test]
async fn t785_capacity_boundaries_publish_without_future_input() {
    for codec in [Codec::H264, Codec::Vp9] {
        for capacity in [8192, 32768, 65536] {
            for size in [32, 8191, 8192, 8193, 32767, 32768, 32769, 65535, 65536, 65537, 262144] {
                ready_packet(codec, capacity, size).await;
            }
        }
    }
}

#[tokio::test]
async fn t785_truncation_and_oversized_packets_remain_rejected() {
    for capacity in [8192, 32768, 65536] {
        let data = fixture(Codec::Vp9, 32, 1).pop().unwrap();
        for end in 0..data.len() {
            if end == 32 { continue; } // complete empty IVF stream.
            let mut reader = BufReader::with_capacity(capacity, &data[..end]);
            assert!(Packetizer::new(Codec::Vp9, Default::default()).read_from(&mut reader).await.is_err());
        }
        for size in [0u32, crate::video_queue::MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
            let mut bad = data.clone();
            bad[32..36].copy_from_slice(&size.to_le_bytes());
            let mut reader = BufReader::with_capacity(capacity, bad.as_slice());
            assert!(Packetizer::new(Codec::Vp9, Default::default()).read_from(&mut reader).await.is_err());
        }
    }
}

#[cfg(unix)]
#[path = "../../../scripts/benchmarks/encoded-reader.rs"]
mod benchmark;
