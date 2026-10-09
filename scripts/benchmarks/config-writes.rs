//! T784: local TCP writes, production packet writer versus the former writer.
//! Run: cargo test --offline -p blent --release --lib t784_tcp -- --ignored --nocapture
use super::StreamServer;
use std::{
    io::{IoSlice, Read, Write},
    net::{TcpListener, TcpStream},
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use tokio::io::{AsyncWrite, AsyncWriteExt};

struct Socket {
    stream: TcpStream,
    writes: usize,
}
impl AsyncWrite for Socket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.writes += 1;
        Poll::Ready(self.stream.write(bytes))
    }
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        self.writes += 1;
        Poll::Ready(self.stream.write_vectored(bytes))
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
async fn sample(size: usize, old: bool) -> serde_json::Value {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    stream.set_nodelay(true).unwrap();
    let (mut receiver, _) = listener.accept().unwrap();
    let mut socket = Socket { stream, writes: 0 };
    let payload = vec![0x5a; size];
    let mut received = vec![0; size + 5];
    let mut values = Vec::new();
    for _ in 0..3000 {
        let started = Instant::now();
        if old {
            socket
                .write_all(&((size + 1) as u32).to_be_bytes())
                .await
                .unwrap();
            socket.write_all(&[0]).await.unwrap();
            socket.write_all(&payload).await.unwrap();
        } else {
            StreamServer::write_packet_bytes(&mut socket, 0, &payload)
                .await
                .unwrap();
        }
        receiver.read_exact(&mut received).unwrap();
        values.push(started.elapsed().as_nanos());
        assert_eq!(&received[5..], &payload);
        assert_eq!(
            &received[..5],
            [&((size + 1) as u32).to_be_bytes()[..], &[0]].concat()
        );
    }
    values.sort_unstable();
    serde_json::json!({"old":old, "payload_bytes":size, "packets":values.len(), "write_syscall_attempts":socket.writes, "receive_p50_ns":values[1500], "receive_p95_ns":values[2850]})
}
#[tokio::test]
#[ignore = "T784 opt-in local TCP timing benchmark"]
async fn t784_tcp() {
    for trial in 0..5 {
        for size in [64, 512, 4096] {
            for old in [trial % 2 == 0, trial % 2 != 0] {
                println!("T784 {}", sample(size, old).await);
            }
        }
    }
}
