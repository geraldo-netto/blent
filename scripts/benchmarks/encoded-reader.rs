//! T785: actual production Packetizer over an OS pipe; no external resources.
//! cargo test --offline -p blent --release --bin blent t785_pipe -- --ignored --nocapture
use super::{fixture, Codec, Packetizer};
use std::{fs::File, io::{Read, Write}, os::fd::FromRawFd, pin::Pin, task::{Context, Poll}, time::Instant};
use tokio::io::{AsyncRead, ReadBuf, BufReader};
struct Source { file: File, reads: usize, short: usize, requested: usize }
impl AsyncRead for Source {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buffer: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let requested = buffer.remaining();
        self.reads += 1;
        self.requested += requested;
        let n = self.file.read(buffer.initialize_unfilled())?;
        self.short += usize::from(n < requested);
        buffer.advance(n);
        Poll::Ready(Ok(()))
    }
}
fn pipe() -> (File, File) {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
    unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) }
}
fn produce(mut output: File, packets: Vec<Vec<u8>>, mode: usize, ack: std::sync::mpsc::Receiver<()>) {
    for data in packets {
        if mode == 2 {
            for chunk in data[..44].chunks(3) { output.write_all(chunk).unwrap(); }
            for chunk in data[44..].chunks(3001) { output.write_all(chunk).unwrap(); }
        } else { output.write_all(&data).unwrap(); }
        if mode != 0 { ack.recv().unwrap(); }
    }
}
async fn sample(codec: Codec, capacity: usize, size: usize, mode: usize) -> serde_json::Value {
    const COUNT: usize = 80;
    let packets = fixture(codec, size, COUNT);
    let (input, output) = pipe();
    let (ack, receive) = std::sync::mpsc::channel();
    let producer = std::thread::spawn(move || produce(output, packets, mode, receive));
    let mut input = BufReader::with_capacity(capacity, Source { file: input, reads: 0, short: 0, requested: 0 });
    let mut parser = Packetizer::new(codec, Default::default());
    let mut times = Vec::new();
    let start = Instant::now();
    for seq in 0..COUNT {
        let begin = Instant::now();
        let (_, frames) = parser.read_from(&mut input).await.unwrap();
        times.push(begin.elapsed().as_nanos());
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].seq as usize, seq);
        assert_eq!(&frames[0].data[frames[0].data.len()-8..], &[0x55; 8]);
        if mode != 0 { ack.send(()).unwrap(); }
    }
    let elapsed = start.elapsed().as_nanos();
    let first = times[0];
    times.sort_unstable();
    producer.join().unwrap();
    serde_json::json!({"codec":codec.wire_name(), "capacity":capacity, "payload":size, "mode":mode,
        "packets":COUNT, "elapsed_ns":elapsed, "first_packet_ns":first, "packet_p50_ns":times[COUNT/2],
        "packet_p95_ns":times[COUNT*95/100], "read_syscalls":input.get_ref().reads,
        "short_reads":input.get_ref().short, "requested_bytes":input.get_ref().requested,
        "buffer_allocation_bytes":capacity, "retained_unread_bytes":input.buffer().len()})
}
#[tokio::test]
#[ignore = "T785 opt-in pipe capacity benchmark"]
async fn t785_pipe() {
    for trial in 0..3 {
        for codec in [Codec::H264, Codec::Vp9] {
            for mode in 0..3 {
                for size in [1024, 32768, 262144] {
                    let capacities = if trial % 2 == 0 { [8192,32768,65536] } else { [65536,32768,8192] };
                    for capacity in capacities { println!("T785 {}", sample(codec, capacity, size, mode).await); }
                }
            }
        }
    }
}
