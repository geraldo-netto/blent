use super::*;
use crate::broker::pump::Pump;
use blent_config::connection::endpoint::Endpoint as Contract;

#[tokio::test]
async fn t878_large_requested_block_adapts_to_small_negotiated_window() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    let io = Arc::new(Mutex::new(Io {
        input: VecDeque::from(*b"x"),
        quantum: 7,
        ..Default::default()
    }));
    let mut pump = Pump::new(
        Script(io.clone()),
        lease.clone(),
        Channel::Camera,
        65536,
        Duration::from_secs(1),
    )
    .unwrap();
    let mut endpoint = rig.host.endpoint(&lease, Channel::Camera);
    assert_eq!(endpoint.identity(), (&lease, Channel::Camera));
    assert!(
        matches!(
            pump.poll(
                &mut Context::from_waker(std::task::Waker::noop()),
                &mut endpoint
            ),
            Poll::Ready(Ok(()))
        ),
        "T878 one byte cannot enter small negotiated window when block allocation is larger"
    );
    assert!(io.lock().unwrap().input.is_empty());
    let frame = rig
        .host
        .transport()
        .unwrap()
        .multiplexer()
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(frame.bytes, b"x");
    rig.host
        .transport()
        .unwrap()
        .multiplexer()
        .finish(true)
        .unwrap();
}

#[tokio::test]
async fn t878_session_adapter_keeps_exact_consumer_and_channel_authority() {
    let mut rig = Rig::new();
    let lease = rig.admit(Resource::Camera);
    assert!(!rig.host.endpoint(&lease, Channel::Control).active());
    assert!(!rig.host.endpoint(&lease, Channel::Management).active());
    rig.peer.offer(Channel::Camera, b"input".to_vec()).unwrap();
    rig.pump(1).unwrap();
    let mut endpoint = rig.host.endpoint(&lease, Channel::Camera);
    let mut bytes = [0; 16];
    let read = endpoint.read(&mut bytes).unwrap().unwrap();
    assert_eq!(&bytes[..read.count], b"input");
    endpoint.consumed(read).unwrap();
    assert!(endpoint.consumed(read).is_err());
    rig.host.cancel(&lease, Reason::Requested).unwrap();
    assert!(!rig.host.endpoint(&lease, Channel::Camera).active());
}
