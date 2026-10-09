//! T784: packet framing, bounded short writes and transport failures.
use super::StreamServer;
use std::{
    io::{ErrorKind, IoSlice},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::AsyncWrite;

struct Sink {
    bytes: Vec<u8>,
    limit: usize,
    calls: usize,
    failure: Option<ErrorKind>,
}
impl Sink {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            calls: 0,
            failure: None,
        }
    }
    fn accept(&mut self, slices: &[IoSlice<'_>]) -> Poll<std::io::Result<usize>> {
        self.calls += 1;
        if let Some(kind) = self.failure.take() {
            return Poll::Ready(Err(kind.into()));
        }
        let before = self.bytes.len();
        self.bytes
            .extend(slices.iter().flat_map(|s| s.iter()).take(self.limit));
        Poll::Ready(Ok(self.bytes.len() - before))
    }
}
impl AsyncWrite for Sink {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.accept(&[IoSlice::new(data)])
    }
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        data: &[IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        self.accept(data)
    }
    fn is_write_vectored(&self) -> bool {
        true
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn t784_packet_bytes_and_bounded_short_writes() {
    for length in 0..=64 {
        let payload: Vec<u8> = (0..length).map(|n| (n * 37) as u8).collect();
        for limit in 1..=70 {
            let mut sink = Sink::new(limit);
            StreamServer::write_packet_bytes(&mut sink, 0, &payload)
                .await
                .unwrap();
            let mut expected = ((length + 1) as u32).to_be_bytes().to_vec();
            expected.push(0);
            expected.extend_from_slice(&payload);
            assert_eq!(sink.bytes, expected);
        }
    }
}

#[tokio::test]
async fn t784_configuration_uses_one_write() {
    for payload in [&b""[..], &b"configuration"[..]] {
        let mut sink = Sink::new(usize::MAX);
        StreamServer::write_packet_bytes(&mut sink, 0, payload)
            .await
            .unwrap();
        assert_eq!(sink.calls, 1);
    }
}

#[tokio::test]
async fn t784_transport_failures_are_not_silenced() {
    for failure in [
        None,
        Some(ErrorKind::Interrupted),
        Some(ErrorKind::BrokenPipe),
    ] {
        let mut sink = Sink::new(0);
        sink.failure = failure;
        let error = StreamServer::write_packet_bytes(&mut sink, 0, b"config")
            .await
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            failure.unwrap_or(ErrorKind::WriteZero)
        );
        assert_eq!(sink.calls, 1);
        assert!(sink.bytes.is_empty());
    }
}

#[tokio::test]
async fn t784_scalar_writer_fallback_preserves_bytes() {
    let mut output = Vec::new(); // AsyncWrite's scalar-only implementation.
    StreamServer::write_packet_bytes(&mut output, 0, b"config")
        .await
        .unwrap();
    assert_eq!(output, b"\0\0\0\x07\0config");
}

#[cfg(unix)]
#[path = "../../../scripts/benchmarks/config-writes.rs"]
mod benchmark;
