//! T933: private real TLS/broker registration carries unchanged camera framing.
use super::*;
use crate::{
    broker::{
        linux::client::Launch,
        transport::{Event as WireEvent, Transport},
    },
    camera::{protocol, usb},
};
use blent_config::{
    camera::{
        connection::{Connection, Invitation},
        CameraProfile,
    },
    connection::{
        management::{Decoder, Message as MediaMessage, Outcome, Profile},
        mux::Multiplexer,
        wire::{Channel, Kind},
    },
};
use tokio::{io::AsyncWriteExt, sync::oneshot};

struct Peer {
    wire: Transport<Box<dyn crate::accessory::Stream>>,
    decoder: Decoder,
    profile: blent_config::connection::management::Camera,
    feedback: Vec<u8>,
    received: Option<oneshot::Sender<Vec<u8>>>,
    revoking: Option<oneshot::Sender<()>>,
    release: Option<oneshot::Receiver<()>>,
}
impl Peer {
    fn new(
        stream: Box<dyn crate::accessory::Stream>,
        profile: blent_config::connection::management::Camera,
        received: oneshot::Sender<Vec<u8>>,
        revoking: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            wire: Transport::new(
                stream,
                Multiplexer::new(1, 1, 4096, 64, 64).unwrap(),
                Duration::from_secs(2),
            )
            .unwrap(),
            decoder: Decoder::new(),
            profile,
            feedback: Vec::new(),
            received: Some(received),
            revoking: Some(revoking),
            release: Some(release),
        }
    }
    fn management(&mut self, message: MediaMessage) {
        let mut bytes = [0; blent_config::connection::management::MAX_RECORD];
        let count = message.encode(&mut bytes).unwrap();
        self.wire
            .multiplexer()
            .offer(Channel::Management, bytes[..count].to_vec())
            .unwrap();
    }
    async fn command(&mut self, message: MediaMessage) {
        match message {
            MediaMessage::Offer(key, credential, Profile::Camera(profile)) => {
                assert_eq!(key.resource, Resource::Camera);
                assert_eq!(credential.as_bytes(), &[0xab; 32]);
                assert_eq!(profile, self.profile);
                self.management(MediaMessage::Prepared(key));
            }
            MediaMessage::Commit(key) => {
                assert_eq!(key.resource, Resource::Camera);
                self.wire
                    .multiplexer()
                    .activate(Channel::Camera, key.generation)
                    .unwrap();
                self.management(MediaMessage::Result(key, Outcome::Admitted));
                let payload = [
                    protocol::MAGIC.as_slice(),
                    "ab".repeat(32).as_bytes(),
                    &[0, 3],
                    &4u32.to_be_bytes(),
                    b"unit",
                ]
                .concat();
                self.wire
                    .multiplexer()
                    .offer(Channel::Camera, payload)
                    .unwrap();
            }
            MediaMessage::Revoke(key, _) => {
                assert_eq!(key.resource, Resource::Camera);
                self.revoking.take().unwrap().send(()).unwrap();
                self.release.take().unwrap().await.unwrap();
                if self
                    .wire
                    .multiplexer()
                    .accepts_data(Channel::Camera, key.generation)
                {
                    self.wire
                        .multiplexer()
                        .close(Channel::Camera, Kind::Close)
                        .unwrap();
                }
                self.wire
                    .multiplexer()
                    .consumer_retired(Channel::Camera, key.generation)
                    .unwrap();
                self.management(MediaMessage::Result(key, Outcome::Revoked));
            }
            _ => panic!("T933 unexpected unrelated resource command"),
        }
    }
    async fn payload(&mut self, channel: Channel, length: u32) {
        let bytes = self.wire.payload().to_vec();
        if channel == Channel::Management {
            let mut offset = 0;
            while offset < bytes.len() {
                let (count, message) = self.decoder.feed(&bytes[offset..]).unwrap();
                offset += count;
                if let Some(message) = message {
                    self.command(message).await;
                }
            }
        } else {
            assert_eq!(channel, Channel::Camera);
            self.feedback.extend_from_slice(&bytes);
            assert!(self.feedback.len() <= 10);
            if self.feedback.len() == 10 {
                self.received
                    .take()
                    .unwrap()
                    .send(self.feedback.clone())
                    .unwrap();
            }
        }
        self.wire.multiplexer().release(channel, length).unwrap();
    }
    async fn run(mut self, mut stopped: oneshot::Receiver<()>) {
        loop {
            tokio::select! {
                _ = &mut stopped => return,
                event = self.wire.next() => {
                    if let WireEvent::Received { header, deliver } = event.unwrap() {
                        if deliver { self.payload(header.channel, header.length).await; }
                        self.wire.consume().unwrap();
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn t933_private_tls_camera_registration_preserves_protocol_and_delayed_retirement() {
    let auth = super::tls::authenticated().await;
    let launch = Launch {
        executable: "/bin/false".into(),
        runtime: auth.fixture.server.settings.runtime.clone(),
        state: auth.fixture.server.settings.state.clone(),
        helper: auth.fixture.server.settings.helper.clone(),
    };
    let profile = CameraProfile {
        usb_peer: Some(auth.pin.as_str().into()),
        ..Default::default()
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let local = tokio::net::TcpListener::from_std(listener.try_clone().unwrap()).unwrap();
    let token = "ab".repeat(32);
    let bridge = usb::native(&profile, &token, listener, launch).unwrap();
    let (server_stop, server_done) = oneshot::channel();
    let running = tokio::spawn(async move {
        let mut fixture = auth.fixture;
        tokio::select! { result=fixture.server.run()=>panic!("server failed: {result:?}"), _=server_done=>() }
        fixture
    });
    let (received, feedback) = oneshot::channel();
    let (revoking, revoked) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let (peer_stop, peer_done) = oneshot::channel();
    let peer = Peer::new(
        auth.tablet,
        profile.invitation_profile().unwrap(),
        received,
        revoking,
        released,
    );
    let tablet = tokio::spawn(peer.run(peer_done));
    bounded(bridge.invite(&token, &profile)).await.unwrap();
    let (mut socket, _) = bounded(local.accept()).await.unwrap();
    assert_eq!(
        bounded(protocol::header(&mut socket, &token))
            .await
            .unwrap(),
        protocol::Header {
            lens: 0,
            rotation: 3
        }
    );
    assert_eq!(
        bounded(protocol::packet(&mut socket)).await.unwrap(),
        b"unit"
    );
    socket.write_all(b"OK").await.unwrap();
    socket.write_u64(1).await.unwrap();
    assert_eq!(
        bounded(feedback).await.unwrap(),
        [b"OK".as_slice(), &1u64.to_be_bytes()].concat()
    );
    assert!(!bridge.retired());
    let close = bridge.close();
    tokio::pin!(close);
    tokio::select! { result=&mut close=>panic!("premature cleanup {result:?}"), result=revoked=>result.unwrap() }
    assert!(!bridge.retired());
    release.send(()).unwrap();
    bounded(&mut close).await.unwrap();
    assert!(bridge.retired());
    peer_stop.send(()).unwrap();
    bounded(tablet).await.unwrap();
    server_stop.send(()).unwrap();
    let mut fixture = bounded(running).await.unwrap();
    assert!(fixture.server.routes.is_empty());
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}
