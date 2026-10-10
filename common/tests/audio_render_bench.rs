//! T818: production queue replay, with exact sample preservation.
use blent_config::audio::{AudioProfile, Direction, PcmBlock, PcmQueue, BLOCK_FRAMES};
use std::time::Instant;

fn replay(direction: Direction, count: usize) -> serde_json::Value {
    let mut profile = AudioProfile::new(direction);
    profile.buffer_ms = 20;
    let mut queue = PcmQueue::new(profile).unwrap();
    let samples: Vec<i16> = (0..BLOCK_FRAMES * direction.channels())
        .map(|i| (i as i16).wrapping_mul(197))
        .collect();
    let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    let block = PcmBlock::from_le_bytes(direction, &bytes).unwrap();
    let mut output = vec![0; samples.len()];
    let mut times = Vec::with_capacity(count);
    queue.push(block.clone(), 0).unwrap();
    let start = Instant::now();
    for _ in 0..count {
        queue.push(block.clone(), 0).unwrap();
        let began = Instant::now();
        let result = queue.render(0, &mut output).unwrap();
        times.push(began.elapsed().as_nanos());
        assert!(!result.silence);
        assert_eq!(std::hint::black_box(&output), &samples);
    }
    let elapsed = start.elapsed().as_nanos();
    times.sort_unstable();
    serde_json::json!({"channels":direction.channels(),"frames":count,"total_ns":elapsed,"p50_ns":times[count/2],"p95_ns":times[count*95/100]})
}
#[test]
fn t818_identity_rate_samples_and_measurements() {
    let count = if std::env::var_os("BLENT_T818_MEASURE").is_some() {
        20000
    } else {
        128
    };
    for direction in [Direction::Microphone, Direction::Speakers] {
        for _ in 0..5 {
            println!("T818_RENDER {}", replay(direction, count));
        }
    }
}
