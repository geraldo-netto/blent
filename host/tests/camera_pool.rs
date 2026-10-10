//! T816: paired allocation/CPU/latency measurements over the actual pool.
#![cfg(target_os = "linux")]
#[path = "../src/camera/frame_pool.rs"]
mod frame_pool;
use std::{
    sync::{atomic::Ordering, Arc},
    time::Instant,
};
use tokio::io::AsyncReadExt;
#[path = "support/count_alloc.rs"]
mod counts;
use counts::{ACTIVE, ALLOCS, BYTES};
fn cpu() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time) },
        0
    );
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}
async fn measure(bytes: usize, pooled: bool, frames: usize) -> serde_json::Value {
    let source = vec![83; bytes];
    let start = Instant::now();
    let mut pool = pooled.then(|| frame_pool::Pool::new(bytes, 3).unwrap());
    let startup = start.elapsed().as_nanos();
    let (published, _) = tokio::sync::watch::channel(None);
    let mut times = Vec::with_capacity(frames);
    ALLOCS.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    let began = cpu();
    ACTIVE.store(true, Ordering::Relaxed);
    for _ in 0..frames {
        let start = Instant::now();
        let mut input = &source[..];
        let frame = if let Some(pool) = &mut pool {
            pool.read(&mut input).await.unwrap().unwrap()
        } else {
            let mut bytes = vec![0; bytes];
            input.read_exact(&mut bytes).await.unwrap();
            Arc::new(bytes)
        };
        assert_eq!(frame[0], 83);
        assert_eq!(frame[bytes - 1], 83);
        std::hint::black_box(published.send_replace(Some(frame)));
        times.push(start.elapsed().as_nanos());
    }
    ACTIVE.store(false, Ordering::Relaxed);
    let elapsed = cpu() - began;
    let allocations = ALLOCS.load(Ordering::Relaxed);
    let allocated = BYTES.load(Ordering::Relaxed);
    if pooled {
        assert_eq!(allocations, 0, "T816: steady decode allocates");
    }
    times.sort_unstable();
    serde_json::json!({"bytes":bytes,"pooled":pooled,"frames":frames,"startup_ns":startup,"cpu_ns":elapsed,"p50_ns":times[frames/2],"p95_ns":times[frames*95/100],"allocations":allocations,"allocated_bytes":allocated})
}
#[tokio::test]
async fn t816_paired_frame_allocation_measurements() {
    let benchmark = std::env::var_os("BLENT_T816_MEASURE").is_some();
    let count = if benchmark { 300 } else { 8 };
    let pairs = if benchmark { 5 } else { 1 };
    for bytes in [1280 * 720 * 3 / 2, 1920 * 1080 * 3 / 2] {
        for pair in 0..pairs {
            for pooled in if pair % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                println!("T816_MEASURE {}", measure(bytes, pooled, count).await);
            }
        }
    }
}
