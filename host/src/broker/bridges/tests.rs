use super::*;
use crate::broker::session::{tests::Rig, Event};
use blent_config::connection::management::{Audio, Camera, Message, Outcome};
use std::{
    collections::VecDeque,
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::io::ReadBuf;

#[derive(Default)]
struct Io {
    input: VecDeque<u8>,
    output: Vec<u8>,
    fail: bool,
    blocked: bool,
}
struct Socket {
    io: Arc<Mutex<Io>>,
    drops: Arc<AtomicUsize>,
}
impl AsyncRead for Socket {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut io = self.io.lock().unwrap();
        if io.input.is_empty() {
            return Poll::Pending;
        }
        let count = io.input.len().min(out.remaining()).min(3);
        for _ in 0..count {
            out.put_slice(&[io.input.pop_front().unwrap()]);
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for Socket {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut io = self.io.lock().unwrap();
        if io.fail {
            return Poll::Ready(Err(io::Error::other("injected local write failure")));
        }
        if io.blocked {
            return Poll::Pending;
        }
        let count = bytes.len().min(2);
        io.output.extend_from_slice(&bytes[..count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
impl Drop for Socket {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
struct Connector {
    polls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    streams: Option<Vec<Box<dyn Stream>>>,
    pending: bool,
    fail: bool,
}
impl Future for Connector {
    type Output = Result<Vec<Box<dyn Stream>>>;
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if self.pending {
            return Poll::Pending;
        }
        if self.fail {
            return Poll::Ready(Err(anyhow::anyhow!("injected connect failure")));
        }
        Poll::Ready(Ok(self.streams.take().unwrap()))
    }
}
impl Drop for Connector {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
struct Probe {
    polls: Arc<AtomicUsize>,
    connector_drops: Arc<AtomicUsize>,
    socket_drops: Arc<AtomicUsize>,
    sockets: Vec<Arc<Mutex<Io>>>,
}
fn connector(count: usize, pending: bool, fail: bool) -> (Connect, Probe) {
    let polls = Arc::new(AtomicUsize::new(0));
    let connector_drops = Arc::new(AtomicUsize::new(0));
    let socket_drops = Arc::new(AtomicUsize::new(0));
    let sockets: Vec<_> = (0..count)
        .map(|_| Arc::new(Mutex::new(Io::default())))
        .collect();
    let streams = sockets
        .iter()
        .map(|io| {
            Box::new(Socket {
                io: io.clone(),
                drops: socket_drops.clone(),
            }) as Box<dyn Stream>
        })
        .collect();
    let connect = Box::pin(Connector {
        polls: polls.clone(),
        drops: connector_drops.clone(),
        streams: Some(streams),
        pending,
        fail,
    });
    (
        connect,
        Probe {
            polls,
            connector_drops,
            socket_drops,
            sockets,
        },
    )
}
fn profile(resource: Resource) -> Profile {
    match resource {
        Resource::DisplaySession => Profile::Display,
        Resource::Camera => Profile::Camera(Camera {
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
        _ => Profile::Audio(Audio {
            processing: 1,
            buffer_ms: 40,
            background: false,
        }),
    }
}
fn register(bridges: &mut Bridges, rig: &mut Rig, resource: Resource, connect: Connect) -> Lease {
    bridges
        .register(
            &mut rig.host,
            Offer {
                resource,
                owner: resource as u64,
                credential: Credential::new([7; 32]),
                profile: profile(resource),
                connect,
            },
            0,
            1000,
        )
        .unwrap()
}
fn poll(bridges: &mut Bridges, rig: &mut Rig) -> Poll<Result<()>> {
    bridges.poll(
        &mut rig.host,
        &mut Context::from_waker(std::task::Waker::noop()),
    )
}
fn dispatch(bridges: &mut Bridges, rig: &mut Rig, now: i64) {
    for event in rig.pump(now).unwrap() {
        if let Event::State(lease, state) = event {
            bridges.state(&mut rig.host, &lease, state).unwrap();
        }
    }
}
fn admit(bridges: &mut Bridges, rig: &mut Rig, lease: &Lease) {
    dispatch(bridges, rig, 0);
    rig.message(Message::Prepared(lease.key));
    dispatch(bridges, rig, 1);
    rig.message(Message::Result(lease.key, Outcome::Admitted));
    dispatch(bridges, rig, 2);
    assert!(rig.host.current(lease));
}

#[tokio::test]
async fn t853_premature_admitted_callback_never_opens_local_socket() {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(65_536, Duration::from_secs(1)).unwrap();
    let (connect, probe) = connector(2, false, false);
    let lease = register(&mut bridges, &mut rig, Resource::DisplaySession, connect);
    let _ = bridges.state(&mut rig.host, &lease, State::Admitted);
    let _ = poll(&mut bridges, &mut rig);
    assert_eq!(
        probe.polls.load(Ordering::SeqCst),
        0,
        "unadmitted resource opened a local socket"
    );
    assert!(!rig.host.current(&lease));
}

#[tokio::test]
async fn t853_all_resource_streams_route_in_descriptor_order_after_admission() {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(65_536, Duration::from_secs(1)).unwrap();
    let mut probes = Vec::new();
    for resource in [
        Resource::DisplaySession,
        Resource::Camera,
        Resource::Microphone,
        Resource::Speakers,
    ] {
        let channels = members(resource);
        let (connect, probe) = connector(channels.len(), false, false);
        let lease = register(&mut bridges, &mut rig, resource, connect);
        assert!(poll(&mut bridges, &mut rig).is_pending());
        assert_eq!(probe.polls.load(Ordering::SeqCst), 0);
        admit(&mut bridges, &mut rig, &lease);
        let _ = poll(&mut bridges, &mut rig);
        assert_eq!(probe.polls.load(Ordering::SeqCst), 1);
        probes.push((resource, probe));
    }
    for (resource, probe) in &probes {
        queue_channel_data(&mut rig, *resource, probe);
    }
    for _ in 0..20 {
        dispatch(&mut bridges, &mut rig, 3);
        assert!(!matches!(poll(&mut bridges, &mut rig), Poll::Ready(Err(_))));
    }
    for (resource, probe) in &probes {
        assert_channel_data(&rig, *resource, probe);
    }
    bridges.disconnect(&mut rig.host).unwrap();
    for (_, probe) in &probes {
        assert_eq!(
            probe.socket_drops.load(Ordering::SeqCst),
            probe.sockets.len()
        );
    }
    finish_disconnect(&mut bridges, &mut rig);
    assert!(bridges.entries.iter().all(Option::is_none));
}
fn queue_channel_data(rig: &mut Rig, resource: Resource, probe: &Probe) {
    for (index, channel) in members(resource).iter().enumerate() {
        rig.peer.offer(*channel, vec![*channel as u8; 11]).unwrap();
        probe.sockets[index]
            .lock()
            .unwrap()
            .input
            .extend(vec![*channel as u8 + 10; 13]);
    }
}
fn assert_channel_data(rig: &Rig, resource: Resource, probe: &Probe) {
    for (index, channel) in members(resource).iter().enumerate() {
        assert_eq!(
            probe.sockets[index].lock().unwrap().output,
            vec![*channel as u8; 11]
        );
        let sent: Vec<_> = rig
            .received
            .iter()
            .filter(|(kind, _)| kind == channel)
            .flat_map(|(_, bytes)| bytes.iter().copied())
            .collect();
        assert_eq!(sent, vec![*channel as u8 + 10; 13]);
    }
}
fn finish_disconnect(bridges: &mut Bridges, rig: &mut Rig) {
    for _ in 0..16 {
        match rig.poll(4) {
            Poll::Ready(Ok(Event::State(lease, state))) => {
                bridges.state(&mut rig.host, &lease, state).unwrap()
            }
            Poll::Ready(Ok(_)) => (),
            _ => return,
        }
    }
    panic!("unbounded disconnect notices");
}

#[tokio::test(start_paused = true)]
async fn t853_pending_connect_timeout_drops_owned_native_state() {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(128, Duration::from_secs(1)).unwrap();
    let (connect, probe) = connector(1, true, false);
    let lease = register(&mut bridges, &mut rig, Resource::Camera, connect);
    admit(&mut bridges, &mut rig, &lease);
    assert!(poll(&mut bridges, &mut rig).is_pending());
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(matches!(poll(&mut bridges, &mut rig), Poll::Ready(Ok(()))));
    assert!(!rig.host.current(&lease));
    assert_eq!(probe.connector_drops.load(Ordering::SeqCst), 1);
    assert_eq!(probe.socket_drops.load(Ordering::SeqCst), 1);
    assert!(bridges.entries[1].as_ref().unwrap().closing);
    assert!(poll(&mut bridges, &mut rig).is_pending());
}

#[tokio::test]
async fn t853_cancel_before_commit_destroys_connector_before_lease_retirement() {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(1, Duration::from_secs(1)).unwrap();
    let (connect, probe) = connector(1, true, false);
    let lease = register(&mut bridges, &mut rig, Resource::Speakers, connect);
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    dispatch(&mut bridges, &mut rig, 1);
    assert_eq!(probe.polls.load(Ordering::SeqCst), 0);
    assert_eq!(probe.connector_drops.load(Ordering::SeqCst), 1);
    assert_eq!(probe.socket_drops.load(Ordering::SeqCst), 1);
    assert!(bridges.entries[3].as_ref().unwrap().closing);
    rig.message(Message::Result(lease.key, Outcome::Revoked));
    dispatch(&mut bridges, &mut rig, 2);
    assert!(bridges.entries[3].is_none());
    let (connect, _probe) = connector(1, false, false);
    let next = register(&mut bridges, &mut rig, Resource::Speakers, connect);
    assert_ne!(lease.key, next.key);
    assert!(bridges
        .state(&mut rig.host, &lease, State::Admitted)
        .is_err());
    assert!(bridges
        .state(&mut rig.host, &lease, State::Retired)
        .is_err());
    assert_eq!(bridges.entries[3].as_ref().unwrap().lease, next);
}

#[tokio::test]
async fn t853_failed_connector_and_slow_socket_preserve_sibling_progress() {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(16, Duration::from_secs(1)).unwrap();
    let (connect, failed) = connector(1, false, true);
    let camera = register(&mut bridges, &mut rig, Resource::Camera, connect);
    admit(&mut bridges, &mut rig, &camera);
    let (connect, slow) = connector(1, false, false);
    let microphone = register(&mut bridges, &mut rig, Resource::Microphone, connect);
    admit(&mut bridges, &mut rig, &microphone);
    slow.sockets[0].lock().unwrap().blocked = true;
    let (connect, fast) = connector(1, false, false);
    let speaker = register(&mut bridges, &mut rig, Resource::Speakers, connect);
    admit(&mut bridges, &mut rig, &speaker);
    assert!(matches!(poll(&mut bridges, &mut rig), Poll::Ready(Ok(()))));
    assert_eq!(failed.socket_drops.load(Ordering::SeqCst), 1);
    rig.peer.offer(Channel::Microphone, vec![1; 8]).unwrap();
    rig.peer.offer(Channel::Speaker, vec![2; 8]).unwrap();
    for _ in 0..8 {
        dispatch(&mut bridges, &mut rig, 3);
        let _ = poll(&mut bridges, &mut rig);
    }
    assert!(slow.sockets[0].lock().unwrap().output.is_empty());
    assert_eq!(fast.sockets[0].lock().unwrap().output, vec![2; 8]);
    assert!(rig.host.current(&speaker));
    assert!(rig.host.current(&microphone));
    assert!(!rig.host.current(&camera));
}

#[tokio::test]
async fn t853_wrong_stream_count_and_local_io_failure_retire_only_owned_resource() {
    for count in [0, 1, 3] {
        wrong_count(count);
    }
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(128, Duration::from_secs(1)).unwrap();
    let (connect, probe) = connector(1, false, false);
    let lease = register(&mut bridges, &mut rig, Resource::Camera, connect);
    admit(&mut bridges, &mut rig, &lease);
    probe.sockets[0].lock().unwrap().fail = true;
    rig.peer.offer(Channel::Camera, vec![1]).unwrap();
    dispatch(&mut bridges, &mut rig, 3);
    assert!(matches!(poll(&mut bridges, &mut rig), Poll::Ready(Ok(()))));
    assert_eq!(probe.socket_drops.load(Ordering::SeqCst), 1);
    assert!(!rig.host.current(&lease));
}
fn wrong_count(count: usize) {
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(128, Duration::from_secs(1)).unwrap();
    let (connect, probe) = connector(count, false, false);
    let lease = register(&mut bridges, &mut rig, Resource::DisplaySession, connect);
    admit(&mut bridges, &mut rig, &lease);
    assert!(matches!(poll(&mut bridges, &mut rig), Poll::Ready(Ok(()))));
    assert_eq!(probe.socket_drops.load(Ordering::SeqCst), count);
    assert!(!rig.host.current(&lease));
}

#[tokio::test]
async fn t853_bridge_invalid_limits_and_duplicate_or_stale_state_are_bounded() {
    for block in [0, 65_537, usize::MAX] {
        assert!(Bridges::new(block, Duration::from_secs(1)).is_err());
    }
    for timeout in [Duration::ZERO, Duration::from_secs(61)] {
        assert!(Bridges::new(1, timeout).is_err());
    }
    let mut rig = Rig::new();
    let mut bridges = Bridges::new(1, Duration::from_secs(60)).unwrap();
    let (connect, _probe) = connector(1, false, false);
    let lease = register(&mut bridges, &mut rig, Resource::Camera, connect);
    assert!(bridges
        .state(&mut rig.host, &lease, State::Retired)
        .is_err());
    bridges
        .state(&mut rig.host, &lease, State::PermissionRequired)
        .unwrap();
    let (connect, duplicate) = connector(1, false, false);
    let offered = Offer {
        resource: Resource::Camera,
        owner: 99,
        credential: Credential::new([1; 32]),
        profile: profile(Resource::Camera),
        connect,
    };
    assert!(bridges.register(&mut rig.host, offered, 0, 1000).is_err());
    assert_eq!(duplicate.socket_drops.load(Ordering::SeqCst), 1);
    admit(&mut bridges, &mut rig, &lease);
    assert!(bridges
        .state(&mut rig.host, &lease, State::Admitted)
        .is_err());
    let mut stale = lease.clone();
    stale.key.request += 1;
    assert!(bridges
        .state(&mut rig.host, &stale, State::Closing)
        .is_err());
    bridges.disconnect(&mut rig.host).unwrap();
    finish_disconnect(&mut bridges, &mut rig);
    assert!(bridges
        .state(&mut rig.host, &lease, State::Closing)
        .is_err());
}
