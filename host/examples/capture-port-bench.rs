extern crate blent;
use blent::{
    capture_contract::VideoOutput,
    media::{EncoderGeneration, VideoPacket},
    media_storage::MediaBytes,
    video_queue,
};
use std::{hint::black_box, time::Instant};
fn main() {
    let (sender, mut receiver) = video_queue::channel(8, Default::default());
    let output: VideoOutput = sender.clone().into();
    let generation = EncoderGeneration::new();
    let packet = VideoPacket {
        data: MediaBytes::from(vec![1; 4096]),
        is_idr: true,
        seq: 1,
        codec_config: None,
        generation: generation.active.clone(),
    };
    for sample in 0..12 {
        let mut results = [0u128; 2];
        for offset in 0..2 {
            let adapter = (sample + offset) % 2;
            let begin = Instant::now();
            for _ in 0..200000 {
                if adapter == 1 {
                    assert!(black_box(&output).send(black_box(packet.clone())).is_ok());
                } else {
                    assert!(black_box(&sender).send(black_box(packet.clone())).is_ok());
                }
                black_box(receiver.try_recv().unwrap());
            }
            results[adapter] = begin.elapsed().as_nanos();
        }
        println!("{} {} {}", sample, results[0], results[1]);
    }
}
