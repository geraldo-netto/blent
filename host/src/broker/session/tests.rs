use super::*;
use blent_config::connection::pairing::PeerId;
use std::{
    io,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::io::ReadBuf;
#[derive(Default)]
struct Io {
    input: VecDeque<u8>,
    output: Vec<u8>,
    flush_blocked: bool,
    quantum: usize,
}
pub(crate) struct Script(Arc<Mutex<Io>>);
impl AsyncRead for Script {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut io = self.0.lock().unwrap();
        if io.input.is_empty() {
            return Poll::Pending;
        }
        let count = buffer.remaining().min(io.input.len()).min(io.quantum);
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
        let count = bytes.len().min(io.quantum);
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
pub(crate) struct Rig {
    pub(crate) host: Session<Script>,
    io: Arc<Mutex<Io>>,
    pub(crate) peer: Multiplexer,
    decoder: Decoder,
    messages: Vec<Message>,
    pub(crate) received: Vec<(Channel, Vec<u8>)>,
}
fn ticket() -> Ticket {
    Ticket {
        epoch: 1,
        peer: PeerId::parse("ab".repeat(32)).unwrap(),
    }
}
impl Rig {
    pub(crate) fn new() -> Self {
        let io = Arc::new(Mutex::new(Io {
            quantum: 7,
            ..Default::default()
        }));
        Self {
            host: Session::new(
                Script(io.clone()),
                ticket(),
                Limits {
                    channel_bytes: 4096,
                    quantum: 17,
                    ..Limits::default()
                },
            )
            .unwrap(),
            io,
            peer: Multiplexer::new(1, 1, 4096, 64, 64).unwrap(),
            decoder: Decoder::new(),
            messages: Vec::new(),
            received: Vec::new(),
        }
    }
    pub(crate) fn poll(&mut self, now: i64) -> Poll<Result<Event>> {
        self.host
            .poll_event(&mut Context::from_waker(std::task::Waker::noop()), now)
    }
    pub(crate) fn pump(&mut self, now: i64) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        for _ in 0..2000 {
            if let Poll::Ready(event) = self.poll(now) {
                events.push(event?);
            }
            self.receive()?;
            self.send()?;
        }
        Ok(events)
    }
    fn send(&mut self) -> Result<()> {
        if let Some(frame) = self.peer.next()? {
            let mut bytes = [0; 40];
            frame.header.encode(&mut bytes)?;
            let mut io = self.io.lock().unwrap();
            io.input.extend(bytes);
            io.input.extend(frame.bytes);
            drop(io);
            self.peer.finish(true)?;
        }
        Ok(())
    }
    fn receive(&mut self) -> Result<()> {
        let mut io = self.io.lock().unwrap();
        if io.output.len() < 40 {
            return Ok(());
        }
        let header = Header::decode(&io.output[..40])?;
        let total = 40 + header.length as usize;
        if io.output.len() < total {
            return Ok(());
        }
        let bytes = io.output.drain(..total).collect::<Vec<_>>();
        drop(io);
        let deliver = self.peer.receive(header)?;
        if deliver && header.channel == Channel::Management {
            self.decode(&bytes[40..])?;
        }
        if deliver && header.channel != Channel::Management {
            self.received.push((header.channel, bytes[40..].to_vec()));
        }
        if deliver {
            self.peer.release(header.channel, header.length)?;
        }
        Ok(())
    }
    fn decode(&mut self, bytes: &[u8]) -> Result<()> {
        let mut offset = 0;
        while offset < bytes.len() {
            let (count, message) = self.decoder.feed(&bytes[offset..])?;
            offset += count;
            if let Some(message) = message {
                if let Message::Commit(key) = &message {
                    for channel in channels::members(key.resource) {
                        self.peer.activate(*channel, key.generation)?;
                    }
                }
                self.messages.push(message);
            }
        }
        Ok(())
    }
    pub(crate) fn message(&mut self, message: Message) {
        let mut bytes = [0; 284];
        let count = message.encode(&mut bytes).unwrap();
        self.peer
            .offer(Channel::Management, bytes[..count].to_vec())
            .unwrap();
    }
    fn register(&mut self, resource: Resource) -> Lease {
        let profile = match resource {
            Resource::DisplaySession => Profile::Display,
            Resource::Camera => Profile::Camera(blent_config::connection::management::Camera {
                width: 640,
                height: 480,
                fps: 30,
                bitrate: 1000,
                min_bitrate: 500,
                freshness_ms: 100,
                lens: 0,
                background: false,
                adaptive: true,
            }),
            _ => Profile::Audio(blent_config::connection::management::Audio {
                processing: 1,
                buffer_ms: 40,
                background: false,
            }),
        };
        self.host
            .register(
                resource,
                resource as u64,
                Credential::new([1; 32]),
                profile,
                0,
                1000,
            )
            .unwrap()
    }
    fn admit(&mut self, resource: Resource) -> Lease {
        let lease = self.register(resource);
        self.pump(0).unwrap();
        self.message(Message::Prepared(lease.key));
        self.pump(1).unwrap();
        assert!(self
            .messages
            .iter()
            .any(|message| matches!(message, Message::Commit(key) if *key == lease.key)));
        self.message(Message::Result(lease.key, Outcome::Admitted));
        self.pump(2).unwrap();
        assert!(self.host.current(&lease));
        lease
    }
}
#[tokio::test]
async fn t872_display_and_camera_have_independent_bounded_consumers() {
    let mut rig = Rig::new();
    let display = rig.admit(Resource::DisplaySession);
    let camera = rig.admit(Resource::Camera);
    rig.peer.offer(Channel::Display, vec![3; 4096]).unwrap();
    rig.peer.offer(Channel::Camera, vec![8; 100]).unwrap();
    rig.pump(3).unwrap();
    let mut bytes = [0; 128];
    let borrow = rig
        .host
        .read(&camera, Channel::Camera, &mut bytes)
        .unwrap()
        .unwrap();
    assert_eq!(borrow.count, 100);
    assert_eq!(&bytes[..100], &[8; 100]);
    assert!(rig.host.read(&camera, Channel::Camera, &mut bytes).is_err());
    assert!(rig
        .host
        .consumed(
            &camera,
            Channel::Camera,
            Read {
                count: 99,
                ..borrow
            }
        )
        .is_err());
    rig.host.consumed(&camera, Channel::Camera, borrow).unwrap();
    rig.host
        .offer(&camera, Channel::Camera, b"uplink reply".to_vec())
        .unwrap();
    rig.pump(4).unwrap();
    assert_eq!(
        rig.host
            .read(&display, Channel::Display, &mut bytes)
            .unwrap()
            .unwrap()
            .count,
        128
    );
    assert!(rig
        .host
        .read(&display, Channel::Camera, &mut bytes)
        .is_err());
    assert!(rig.host.offer(&display, Channel::Camera, vec![1]).is_err());
    assert!(rig
        .host
        .offer(&display, Channel::Display, Vec::new())
        .is_err());
}
#[tokio::test]
async fn t872_revoke_waits_for_peer_and_actual_native_retirement() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::DisplaySession);
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    rig.pump(3).unwrap();
    rig.message(Message::Result(lease.key, Outcome::Revoked));
    rig.pump(4).unwrap();
    assert!(!rig.host.current(&lease));
    assert!(rig.host.slots[0].is_some());
    rig.host.native_retired(&lease).unwrap();
    assert!(rig.host.slots[0].is_none());
    for channel in channels::members(Resource::DisplaySession) {
        rig.peer.consumer_retired(*channel, 1).unwrap();
        rig.peer.replace_channel(*channel, 2).unwrap();
    }
    let replacement = rig.register(Resource::DisplaySession);
    assert_eq!(replacement.key.generation, 2);
    assert!(rig.host.native_retired(&lease).is_err());
    assert!(rig.host.cancel(&lease, Reason::Requested).is_err());
}
#[tokio::test]
async fn t872_disconnect_retains_resource_until_native_callback() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.host.disconnect().unwrap();
    assert!(rig.host.slots[1].is_some());
    assert!(!rig.host.current(&lease));
    assert!(rig.host.offer(&lease, Channel::Camera, vec![1]).is_err());
    rig.host.native_retired(&lease).unwrap();
    assert!(rig.host.slots[1].is_none());
}
#[tokio::test]
async fn t872_expired_commit_drains_without_double_close() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::DisplaySession);
    rig.pump(0).unwrap();
    rig.message(Message::Prepared(lease.key));
    // Parse Prepared and queue Commit, without permitting any Commit output yet.
    for _ in 0..200 {
        rig.send().unwrap();
        if let Poll::Ready(result) = rig.poll(1) {
            result.unwrap();
        }
        if rig
            .host
            .publications
            .iter()
            .any(|p| matches!(p.effect, Effect::Commit(_)))
        {
            break;
        }
    }
    let result = rig.pump(1001);
    assert!(
        result.is_ok(),
        "T872 expired Commit double-close: {:?}",
        result.err()
    );
    assert!(!rig.host.current(&lease));
    assert!(rig.host.binding(&lease).unwrap().closing);
}
#[tokio::test]
async fn t872_peer_media_terminal_revokes_resource() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.peer.close(Channel::Camera, Kind::Reset).unwrap();
    rig.pump(3).unwrap();
    assert!(
        !rig.host.current(&lease),
        "T872 peer terminal left resource admitted"
    );
}

#[tokio::test]
async fn t872_pre_admission_payload_becomes_readable_on_admission() {
    let mut rig = Rig::new();
    let lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.message(Message::Prepared(lease.key));
    rig.pump(1).unwrap();
    rig.peer.offer(Channel::Camera, b"early".to_vec()).unwrap();
    let events = rig.pump(2).unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::Readable(..))));
    rig.message(Message::Result(lease.key, Outcome::Admitted));
    let events = rig.pump(3).unwrap();
    assert!(
        events.contains(&Event::Readable(lease.key, Channel::Camera)),
        "T872 pre-admission bytes lost readiness notification"
    );
}
#[tokio::test]
async fn t872_disconnect_settles_already_retired_native_consumer() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    rig.host.native_retired(&lease).unwrap();
    assert!(rig.host.slots[1].is_some());
    rig.host.disconnect().unwrap();
    assert!(
        rig.host.slots[1].is_none(),
        "T872 disconnect retained fully retired resource"
    );
}
#[tokio::test(start_paused = true)]
async fn t872_awaited_event_uses_current_time_for_resource_deadline() {
    use std::future::Future;
    let mut rig = Rig::new();
    let lease = rig.register(Resource::DisplaySession);
    rig.pump(0).unwrap();
    rig.message(Message::Prepared(lease.key));
    rig.send().unwrap();
    let bytes = std::mem::take(&mut rig.io.lock().unwrap().input);
    let mut future = Box::pin(rig.host.next(1));
    let mut cx = Context::from_waker(std::task::Waker::noop());
    assert!(future.as_mut().poll(&mut cx).is_pending());
    tokio::time::advance(Duration::from_millis(1001)).await;
    rig.io.lock().unwrap().input = bytes;
    let mut completed = false;
    for _ in 0..100 {
        if let Poll::Ready(result) = future.as_mut().poll(&mut cx) {
            result.unwrap();
            completed = true;
            break;
        }
    }
    assert!(completed);
    drop(future);
    assert!(
        rig.host.binding(&lease).unwrap().closing,
        "T872 wait reused pre-expiry clock"
    );
}
#[tokio::test]
async fn t872_premature_media_terminal_cannot_corrupt_inactive_generation() {
    let mut rig = Rig::new();
    let _lease = rig.register(Resource::Camera);
    rig.pump(0).unwrap();
    rig.peer.close(Channel::Camera, Kind::Reset).unwrap();
    assert!(
        rig.pump(1).is_err(),
        "T872 pre-Commit terminal silently closed uninstalled generation"
    );
}

mod consumer;

mod barriers;

pub(crate) mod tls;

mod pump;
