use super::*;
use blent_config::connection::wire::{Channel, Kind};
use std::{collections::VecDeque, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadBuf};

#[derive(Default)]
struct Script {
    input: VecDeque<u8>,
    output: Vec<u8>,
    quantum: usize,
    eof: bool,
    read_error: bool,
    write_error: bool,
    block_write: bool,
    block_flush: bool,
    zero_write: bool,
}
impl AsyncRead for Script {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.read_error {
            return Poll::Ready(Err(io::Error::other("read failure")));
        }
        if self.input.is_empty() && !self.eof {
            return Poll::Pending;
        }
        let count = self.quantum.min(buf.remaining()).min(self.input.len());
        for _ in 0..count {
            buf.put_slice(&[self.input.pop_front().unwrap()]);
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for Script {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.write_error {
            return Poll::Ready(Err(io::Error::other("write failure")));
        }
        if self.block_write {
            return Poll::Pending;
        }
        let count = if self.zero_write {
            0
        } else {
            bytes.len().min(self.quantum)
        };
        self.output.extend_from_slice(&bytes[..count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.block_flush {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
fn mux() -> Multiplexer {
    Multiplexer::new(7, 1, 128, 8, 64).unwrap()
}
fn driver() -> Transport<Script> {
    Transport::new(
        Script {
            quantum: 3,
            ..Default::default()
        },
        mux(),
        Duration::from_millis(50),
    )
    .unwrap()
}
fn header(kind: Kind, sequence: u64, length: u32, credit: u32) -> Header {
    Header {
        kind,
        channel: Channel::Management,
        session: 7,
        generation: 1,
        sequence,
        length,
        credit,
    }
}
fn feed(driver: &mut Transport<Script>, header: Header, body: &[u8]) {
    let mut bytes = [0; HEADER_BYTES];
    header.encode(&mut bytes).unwrap();
    driver.stream.input.extend(bytes);
    driver.stream.input.extend(body);
}
fn poll(driver: &mut Transport<Script>) -> Poll<Result<Event>> {
    let mut cx = Context::from_waker(std::task::Waker::noop());
    driver.poll_event(&mut cx)
}
fn complete(driver: &mut Transport<Script>) -> Result<Event> {
    for _ in 0..1000 {
        if let Poll::Ready(result) = poll(driver) {
            return result;
        }
    }
    panic!("T873 bounded script did not complete");
}
fn initial(driver: &mut Transport<Script>) {
    assert_eq!(
        complete(driver).unwrap(),
        Event::Written(header(Kind::Credit, 0, 0, 128))
    );
}
#[tokio::test]
async fn t873_fragmented_reads_writes_preserve_reservations_and_payload() {
    let mut driver = driver();
    initial(&mut driver);
    feed(&mut driver, header(Kind::Credit, 0, 0, 128), &[]);
    assert!(matches!(
        complete(&mut driver).unwrap(),
        Event::Received { deliver: false, .. }
    ));
    assert!(driver.payload().is_empty());
    driver.consume().unwrap();
    driver
        .multiplexer()
        .offer(Channel::Management, b"abcdef".to_vec())
        .unwrap();
    assert!(poll(&mut driver).is_pending());
    let pointer = driver.mux.reserved().unwrap().bytes.as_ptr();
    assert_eq!(
        complete(&mut driver).unwrap(),
        Event::Written(header(Kind::Data, 1, 6, 0))
    );
    assert!(!pointer.is_null());
    assert!(driver.mux.reserved().is_none());
    assert_eq!(&driver.stream.output[2 * HEADER_BYTES..], b"abcdef");
    feed(&mut driver, header(Kind::Data, 1, 7, 0), b"payload");
    assert_eq!(
        complete(&mut driver).unwrap(),
        Event::Received {
            header: header(Kind::Data, 1, 7, 0),
            deliver: true
        }
    );
    assert_eq!(driver.payload(), b"payload");
    assert!(poll(&mut driver).is_pending());
    driver.consume().unwrap();
    assert!(driver.payload().is_empty());
    assert!(driver.consume().is_err());
    driver
        .multiplexer()
        .release(Channel::Management, 7)
        .unwrap();
    assert_eq!(
        complete(&mut driver).unwrap(),
        Event::Written(header(Kind::Credit, 2, 0, 7))
    );
}
#[tokio::test(start_paused = true)]
async fn t873_cancellation_does_not_restart_partial_read_deadline() {
    let mut driver = driver();
    initial(&mut driver);
    driver.stream.input.push_back(b'B');
    assert!(poll(&mut driver).is_pending());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), driver.next())
            .await
            .is_err()
    );
    driver.stream.input.push_back(b'L');
    assert!(poll(&mut driver).is_pending());
    tokio::time::advance(Duration::from_millis(31)).await;
    assert!(complete(&mut driver)
        .unwrap_err()
        .to_string()
        .contains("receive frame deadline"));
    assert!(complete(&mut driver).is_err());
}
#[tokio::test(start_paused = true)]
async fn t873_idle_read_has_no_deadline_but_partial_body_does() {
    let mut driver = driver();
    initial(&mut driver);
    tokio::time::advance(Duration::from_secs(100)).await;
    assert!(poll(&mut driver).is_pending());
    driver.stream.quantum = HEADER_BYTES;
    feed(&mut driver, header(Kind::Data, 0, 3, 0), b"x");
    assert!(poll(&mut driver).is_pending());
    tokio::time::advance(Duration::from_millis(51)).await;
    assert!(complete(&mut driver).is_err());
}
#[tokio::test(start_paused = true)]
async fn t873_writer_deadline_includes_flush_and_cancelled_waits() {
    for flush in [false, true] {
        let mut driver = driver();
        driver.stream.block_flush = flush;
        driver.stream.block_write = !flush;
        for _ in 0..40 {
            assert!(poll(&mut driver).is_pending());
        }
        assert!(driver.mux.reserved().is_some());
        assert!(
            tokio::time::timeout(Duration::from_millis(51), driver.next())
                .await
                .unwrap()
                .is_err()
        );
        assert!(driver.mux.reserved().is_none());
        assert!(driver
            .multiplexer()
            .offer(Channel::Management, vec![1])
            .is_err());
        assert!(driver.consume().is_err());
    }
}
#[tokio::test]
async fn t873_pending_write_services_read_and_pending_consumer_services_write() {
    let mut driver = driver();
    initial(&mut driver);
    feed(&mut driver, header(Kind::Credit, 0, 0, 128), &[]);
    complete(&mut driver).unwrap();
    driver.consume().unwrap();
    driver
        .multiplexer()
        .offer(Channel::Management, vec![8; 10])
        .unwrap();
    driver.stream.block_write = true;
    feed(&mut driver, header(Kind::Data, 1, 3, 0), b"abc");
    assert!(matches!(
        complete(&mut driver).unwrap(),
        Event::Received { deliver: true, .. }
    ));
    assert!(driver.mux.reserved().is_some());
    driver.stream.block_write = false;
    assert!(matches!(complete(&mut driver).unwrap(), Event::Written(_)));
    assert_eq!(driver.payload(), b"abc");
    driver.consume().unwrap();
}
#[tokio::test]
async fn t873_invalid_headers_eof_and_native_errors_poison_connection() {
    for mode in 0..6 {
        let mut driver = driver();
        initial(&mut driver);
        match mode {
            0 => driver.stream.input.extend([0; HEADER_BYTES]),
            1 => driver.stream.eof = true,
            2 => driver.stream.read_error = true,
            3 => {
                driver.stream.write_error = true;
                driver.mux.close(Channel::Management, Kind::Close).unwrap();
            }
            4 => {
                driver.stream.zero_write = true;
                driver.mux.close(Channel::Management, Kind::Close).unwrap();
            }
            _ => {
                feed(&mut driver, header(Kind::Data, 0, 2, 0), b"x");
                driver.stream.eof = true;
            }
        }
        assert!(complete(&mut driver).is_err());
        assert!(complete(&mut driver).is_err());
    }
}
#[tokio::test]
async fn t873_deadline_bounds_and_closed_channel_drain_are_explicit() {
    for millis in [0, 60001, u64::MAX] {
        assert!(Transport::new(Script::default(), mux(), Duration::from_millis(millis)).is_err());
    }
    let mut driver = driver();
    initial(&mut driver);
    driver
        .multiplexer()
        .close(Channel::Management, Kind::Close)
        .unwrap();
    complete(&mut driver).unwrap();
    feed(&mut driver, header(Kind::Data, 0, 2, 0), b"xx");
    assert!(matches!(
        complete(&mut driver).unwrap(),
        Event::Received { deliver: false, .. }
    ));
    driver.consume().unwrap();
}
#[tokio::test]
async fn t873_real_duplex_handles_bidirectional_fragmentation() {
    let (local, mut remote) = tokio::io::duplex(3);
    let mut driver = Transport::new(local, mux(), Duration::from_secs(1)).unwrap();
    let task = tokio::spawn(async move {
        let mut bytes = [0; HEADER_BYTES];
        remote.read_exact(&mut bytes).await.unwrap();
        assert_eq!(Header::decode(&bytes).unwrap().kind, Kind::Credit);
        header(Kind::Data, 0, 6, 0).encode(&mut bytes).unwrap();
        remote.write_all(&bytes).await.unwrap();
        remote.write_all(b"duplex").await.unwrap();
        remote
    });
    assert!(matches!(driver.next().await.unwrap(), Event::Written(_)));
    assert!(matches!(
        driver.next().await.unwrap(),
        Event::Received { deliver: true, .. }
    ));
    assert_eq!(driver.payload(), b"duplex");
    driver.consume().unwrap();
    let _remote = task.await.unwrap();
}

#[tokio::test]
async fn t873_revocation_during_body_must_drain_completed_frame() {
    let mut driver = driver();
    initial(&mut driver);
    driver.stream.quantum = HEADER_BYTES;
    feed(&mut driver, header(Kind::Data, 0, 3, 0), b"x");
    assert!(poll(&mut driver).is_pending());
    driver
        .multiplexer()
        .close(Channel::Management, Kind::Close)
        .unwrap();
    driver.stream.input.extend(b"yz");
    let mut received = false;
    for _ in 0..2 {
        if let Event::Received { deliver, .. } = complete(&mut driver).unwrap() {
            assert!(
                !deliver,
                "T873 revoked partial frame regained delivery authority"
            );
            received = true;
            break;
        }
    }
    assert!(received);
}
#[tokio::test]
async fn t873_payload_is_unavailable_until_complete_frame_event() {
    let mut driver = driver();
    initial(&mut driver);
    driver.stream.quantum = HEADER_BYTES;
    feed(&mut driver, header(Kind::Data, 0, 3, 0), b"x");
    assert!(poll(&mut driver).is_pending());
    assert!(
        driver.payload().is_empty(),
        "T873 unfinished frame exposes staging bytes"
    );
}

#[tokio::test]
async fn t873_stopping_transfers_stream_for_explicit_shutdown() {
    let mut driver = driver();
    initial(&mut driver);
    let mut stream = driver.into_stream();
    stream.shutdown().await.unwrap();
    assert_eq!(stream.output.len(), HEADER_BYTES);
}
#[tokio::test]
async fn t873_bounded_invalid_declared_lengths_never_reach_allocation() {
    for length in [0, MAX_PAYLOAD + 1, u32::MAX, 1 << 31] {
        let mut driver = driver();
        initial(&mut driver);
        let mut bytes = [0; HEADER_BYTES];
        header(Kind::Data, 0, 1, 0).encode(&mut bytes).unwrap();
        bytes[32..36].copy_from_slice(&length.to_be_bytes());
        driver.stream.input.extend(bytes);
        assert!(complete(&mut driver).is_err());
        assert!(driver.payload().is_empty());
    }
    for generation in 2..258 {
        let mut driver = driver();
        initial(&mut driver);
        let mut stale = header(Kind::Data, 0, 1, 0);
        stale.generation = generation;
        feed(&mut driver, stale, b"x");
        assert!(complete(&mut driver).is_err());
    }
}
#[tokio::test]
async fn t873_borrowed_reservation_preserves_original_buffer_and_revocation() {
    let mut mux = mux();
    assert!(!mux.accepts_data(Channel::Display, 1));
    assert!(!mux.accepts_data(Channel::Management, 2));
    mux.next_frame().unwrap().unwrap();
    mux.finish(true).unwrap();
    mux.receive(header(Kind::Credit, 0, 0, 128)).unwrap();
    let bytes = vec![1; 12];
    let pointer = bytes.as_ptr();
    mux.offer(Channel::Management, bytes).unwrap();
    mux.next_frame().unwrap().unwrap();
    assert_eq!(mux.reserved().unwrap().bytes.as_ptr(), pointer);
    mux.close(Channel::Management, Kind::Close).unwrap();
    assert_eq!(mux.reserved().unwrap().bytes.as_ptr(), pointer);
    assert!(!mux.accepts_data(Channel::Management, 1));
    mux.finish(true).unwrap();
    assert!(mux.reserved().is_none());
}
#[tokio::test]
async fn t872_receive_gate_rejects_invalid_generations_without_mutation() {
    let mut driver = driver();
    assert!(driver.hold(Channel::Management, None).is_err());
    for value in [0, blent_config::connection::wire::MAX_COUNTER + 1, u64::MAX] {
        assert!(
            driver.hold(Channel::Camera, Some(value)).is_err(),
            "T872 invalid receive gate generation accepted"
        );
        assert_eq!(driver.held[Channel::Camera as usize - 1], None);
    }
    driver.hold(Channel::Camera, Some(1)).unwrap();
    driver.hold(Channel::Camera, None).unwrap();
}
