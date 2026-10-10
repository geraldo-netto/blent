use super::*;
use blent_config::connection::{
    endpoint::Read,
    management::{Key, Resource},
    pairing::{PeerId, Ticket},
};
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::io::ReadBuf;

fn lease() -> Lease {
    Lease {
        connection: Ticket {
            epoch: 3,
            peer: PeerId::parse("ab".repeat(32)).unwrap(),
        },
        key: Key {
            resource: Resource::Camera,
            request: 2,
            generation: 1,
        },
        owner: 7,
    }
}
struct Channel {
    lease: Lease,
    active: bool,
    capacity: usize,
    incoming: VecDeque<u8>,
    borrowed: Option<Read>,
    next: u64,
    completed: usize,
    outgoing: Vec<Vec<u8>>,
    blocked: bool,
    bad: Option<Read>,
}
impl Channel {
    fn new() -> Self {
        Self {
            lease: lease(),
            active: true,
            capacity: 4096,
            incoming: VecDeque::new(),
            borrowed: None,
            next: 1,
            completed: 0,
            outgoing: Vec::new(),
            blocked: false,
            bad: None,
        }
    }
}
impl Endpoint for Channel {
    fn capacity(&self) -> usize {
        self.capacity
    }
    fn identity(&self) -> (&Lease, super::Channel) {
        (&self.lease, super::Channel::Camera)
    }
    fn active(&self) -> bool {
        self.active
    }
    fn offer(&mut self, bytes: Vec<u8>) -> std::result::Result<(), Vec<u8>> {
        if self.blocked || bytes.capacity() > self.capacity {
            Err(bytes)
        } else {
            self.outgoing.push(bytes);
            Ok(())
        }
    }
    fn read(&mut self, out: &mut [u8]) -> Result<Option<Read>> {
        if self.bad.is_some() {
            return Ok(self.bad);
        }
        assert!(self.borrowed.is_none());
        let count = out.len().min(self.incoming.len());
        if count == 0 {
            return Ok(None);
        }
        for byte in &mut out[..count] {
            *byte = self.incoming.pop_front().unwrap();
        }
        let read = Read {
            id: self.next,
            count,
        };
        self.next += 1;
        self.borrowed = Some(read);
        Ok(Some(read))
    }
    fn consumed(&mut self, read: Read) -> Result<()> {
        assert_eq!(self.borrowed.take(), Some(read));
        self.completed += read.count;
        Ok(())
    }
}
#[derive(Default)]
struct Io {
    input: VecDeque<u8>,
    output: Vec<u8>,
    flush_blocked: bool,
    eof: bool,
    write_zero: bool,
    write_blocked: bool,
    failure: bool,
}
struct Script(Arc<Mutex<Io>>);
impl AsyncRead for Script {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut io = self.0.lock().unwrap();
        if io.failure {
            return Poll::Ready(Err(io::Error::other("fixture failure")));
        }
        if io.input.is_empty() && !io.eof {
            return Poll::Pending;
        }
        let count = io.input.len().min(buffer.remaining()).min(3);
        for _ in 0..count {
            buffer.put_slice(&[io.input.pop_front().unwrap()]);
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for Script {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut io = self.0.lock().unwrap();
        if io.failure {
            return Poll::Ready(Err(io::Error::other("fixture failure")));
        }
        if io.write_blocked {
            return Poll::Pending;
        }
        let count = if io.write_zero { 0 } else { bytes.len().min(3) };
        io.output.extend_from_slice(&bytes[..count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.0.lock().unwrap().flush_blocked {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
fn rig() -> (Pump<Script>, Arc<Mutex<Io>>, Channel) {
    let io = Arc::new(Mutex::new(Io::default()));
    (
        Pump::new(
            Script(io.clone()),
            lease(),
            super::Channel::Camera,
            16,
            Duration::from_secs(1),
        )
        .unwrap(),
        io,
        Channel::new(),
    )
}
fn poll(pump: &mut Pump<Script>, channel: &mut Channel) -> Poll<Result<()>> {
    pump.poll(&mut Context::from_waker(std::task::Waker::noop()), channel)
}
fn drive(pump: &mut Pump<Script>, channel: &mut Channel) {
    for _ in 0..128 {
        match poll(pump, channel) {
            Poll::Ready(result) => result.unwrap(),
            Poll::Pending => return,
        }
    }
    panic!("T878 bridge failed to reach bounded idle state");
}

#[tokio::test]
async fn t878_partial_duplex_keeps_credit_until_flush_and_reuses_buffers() {
    let (mut pump, io, mut channel) = rig();
    channel.incoming.extend(b"remote message spanning blocks");
    io.lock()
        .unwrap()
        .input
        .extend(b"local message spanning blocks");
    io.lock().unwrap().flush_blocked = true;
    drive(&mut pump, &mut channel);
    assert_eq!(channel.completed, 0);
    assert_eq!(io.lock().unwrap().output, b"remote message s");
    assert_eq!(channel.outgoing.concat(), b"local message spanning blocks");
    let allocation = pump.receive.bytes.as_ptr();
    io.lock().unwrap().flush_blocked = false;
    drive(&mut pump, &mut channel);
    assert_eq!(channel.completed, 30);
    assert_eq!(io.lock().unwrap().output, b"remote message spanning blocks");
    assert_eq!(pump.receive.bytes.as_ptr(), allocation);
    assert!(poll(&mut pump, &mut channel).is_pending());
}

#[tokio::test]
async fn t878_backpressure_preserves_exact_block_while_receive_continues() {
    let (mut pump, io, mut channel) = rig();
    channel.blocked = true;
    channel.incoming.extend(b"other direction");
    io.lock().unwrap().input.extend(b"one unsent block");
    drive(&mut pump, &mut channel);
    assert_eq!(io.lock().unwrap().output, b"other direction");
    assert_eq!(channel.completed, 15);
    assert!(channel.outgoing.is_empty());
    assert_eq!(io.lock().unwrap().input.len(), 13);
    channel.blocked = false;
    drive(&mut pump, &mut channel);
    assert_eq!(channel.outgoing.concat(), b"one unsent block");
}

#[tokio::test(start_paused = true)]
async fn t878_absolute_pending_deadlines_do_not_refresh_or_charge_idle_reads() {
    let (mut pump, io, mut channel) = rig();
    assert!(poll(&mut pump, &mut channel).is_pending());
    tokio::time::advance(Duration::from_secs(10)).await;
    assert!(poll(&mut pump, &mut channel).is_pending());
    channel.blocked = true;
    io.lock().unwrap().input.extend(b"local");
    assert!(poll(&mut pump, &mut channel).is_pending());
    tokio::time::advance(Duration::from_millis(900)).await;
    assert!(poll(&mut pump, &mut channel).is_pending());
    tokio::time::advance(Duration::from_millis(100)).await;
    assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
    channel.blocked = false;
    assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
    assert!(channel.outgoing.is_empty());
}

#[tokio::test(start_paused = true)]
async fn t878_stalled_consumer_expires_without_credit_or_sibling_failure() {
    let (mut pump, io, mut channel) = rig();
    channel.incoming.extend(b"waiting");
    io.lock().unwrap().flush_blocked = true;
    drive(&mut pump, &mut channel);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
    assert_eq!(channel.completed, 0);
    let (mut sibling, other, mut endpoint) = rig();
    endpoint.incoming.extend(b"independent");
    drive(&mut sibling, &mut endpoint);
    assert_eq!(other.lock().unwrap().output, b"independent");
}

#[tokio::test]
async fn t878_revoked_or_rebound_lease_cannot_complete_pending_consumer() {
    for rebound in [false, true] {
        let (mut pump, io, mut channel) = rig();
        channel.incoming.extend(b"pending");
        io.lock().unwrap().write_blocked = true;
        assert!(poll(&mut pump, &mut channel).is_pending());
        if rebound {
            channel.lease.connection.epoch += 1;
        } else {
            channel.active = false;
        }
        io.lock().unwrap().write_blocked = false;
        assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
        assert!(io.lock().unwrap().output.is_empty());
        assert_eq!(channel.completed, 0);
    }
}

#[tokio::test]
async fn t878_invalid_extents_eof_and_native_errors_fail_closed() {
    for bad in [
        Read { id: 0, count: 1 },
        Read { id: 1, count: 0 },
        Read { id: 1, count: 17 },
        Read {
            id: u64::MAX,
            count: usize::MAX,
        },
    ] {
        let (mut pump, _, mut channel) = rig();
        channel.bad = Some(bad);
        assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
    }
    for mode in 0..4 {
        let (mut pump, io, mut channel) = rig();
        {
            let mut io = io.lock().unwrap();
            io.eof = mode == 0;
            io.write_zero = mode == 1;
            io.failure = mode >= 2;
        }
        if mode == 1 || mode == 2 {
            channel.incoming.extend(b"fail");
        }
        assert!(matches!(poll(&mut pump, &mut channel), Poll::Ready(Err(_))));
        assert_eq!(channel.completed, 0);
    }
}

#[test]
fn t878_invalid_block_and_deadline_rejected_before_io() {
    for block in [0, 1, 65536, 65537, usize::MAX] {
        let io = Script(Arc::new(Mutex::new(Io::default())));
        assert_eq!(
            Pump::new(
                io,
                lease(),
                super::Channel::Camera,
                block,
                Duration::from_secs(1)
            )
            .is_ok(),
            (1..=65536).contains(&block)
        );
    }
    for timeout in [
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_secs(60),
        Duration::MAX,
    ] {
        let io = Script(Arc::new(Mutex::new(Io::default())));
        assert_eq!(
            Pump::new(io, lease(), super::Channel::Camera, 1, timeout).is_ok(),
            !timeout.is_zero() && timeout <= Duration::from_secs(60)
        );
    }
}

#[tokio::test]
async fn t878_tiny_endpoint_capacity_never_charges_hidden_vector_growth() {
    for capacity in [1, 2, 4, 7, 8, 4096, 65536] {
        let (mut pump, io, mut endpoint) = rig();
        endpoint.capacity = capacity;
        io.lock().unwrap().input.push_back(42);
        assert!(
            matches!(poll(&mut pump, &mut endpoint), Poll::Ready(Ok(()))),
            "T878 allocation exceeded endpoint capacity {capacity}"
        );
        assert_eq!(endpoint.outgoing.concat(), [42]);
    }
}

#[tokio::test]
async fn t878_native_socket_duplex_and_drop_retire_exact_local_handle() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (client, accepted) = tokio::join!(
            TcpStream::connect(listener.local_addr().unwrap()),
            listener.accept()
        );
        let mut client = client.unwrap();
        let mut endpoint = Channel::new();
        endpoint.incoming.extend(b"remote bytes");
        let mut pump = Pump::new(
            accepted.unwrap().0,
            lease(),
            super::Channel::Camera,
            65536,
            Duration::from_secs(1),
        )
        .unwrap();
        let produce = async {
            client.write_all(b"local bytes").await.unwrap();
            let mut received = [0; 12];
            client.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"remote bytes");
        };
        let transfer = async {
            while endpoint.completed < 12 || endpoint.outgoing.concat().len() < 11 {
                std::future::poll_fn(|cx| pump.poll(cx, &mut endpoint))
                    .await
                    .unwrap();
            }
        };
        tokio::join!(produce, transfer);
        assert_eq!(endpoint.outgoing.concat(), b"local bytes");
        drop(pump);
        assert_eq!(client.read(&mut [0]).await.unwrap(), 0);
    })
    .await
    .unwrap();
}
