//! T992: unchanged audio bootstrap/clock/PCM over real private mutual TLS routing.
use super::*;
use crate::{
    audio::{self, usb},
    audio_control::Report,
    broker::{
        linux::client::Launch,
        transport::{Event as WireEvent, Transport},
    },
};
use blent_config::{
    audio::{AudioOptions, Direction},
    connection::{
        management::{Decoder, Message as MediaMessage, Outcome, Profile},
        media::Connection,
        mux::Multiplexer,
        wire::{Channel, Kind},
    },
};
use tokio::sync::{oneshot, watch};

struct Peer {
    wire: Transport<Box<dyn crate::accessory::Stream>>,
    decoder: Decoder,
    profile: blent_config::connection::management::Audio,
    key: Option<Key>,
    bytes: Vec<u8>,
    received: Option<oneshot::Sender<Vec<u8>>>,
    revoking: Option<oneshot::Sender<()>>,
    release: oneshot::Receiver<()>,
    held: Option<Key>,
}
impl Peer {
    fn new(
        stream: Box<dyn crate::accessory::Stream>,
        profile: blent_config::connection::management::Audio,
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
            key: None,
            bytes: Vec::new(),
            received: Some(received),
            revoking: Some(revoking),
            release,
            held: None,
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
    fn command(&mut self, message: MediaMessage) {
        match message {
            MediaMessage::Offer(key, credential, Profile::Audio(profile)) => {
                assert_eq!(key.resource, Resource::Speakers);
                assert_eq!(credential.as_bytes(), &[0xab; 32]);
                assert_eq!(profile, self.profile);
                self.key = Some(key);
                self.management(MediaMessage::Prepared(key));
            }
            MediaMessage::Commit(key) => {
                assert_eq!(Some(key), self.key);
                self.wire
                    .multiplexer()
                    .activate(Channel::Speaker, key.generation)
                    .unwrap();
                self.management(MediaMessage::Result(key, Outcome::Admitted));
                let request = [
                    b"BLAUREQ2".as_slice(),
                    "ab".repeat(32).as_bytes(),
                    &[7, 0, 2, 2],
                ]
                .concat();
                self.wire
                    .multiplexer()
                    .offer(Channel::Speaker, request)
                    .unwrap();
            }
            MediaMessage::Revoke(key, _) => {
                assert_eq!(Some(key), self.key);
                self.held = Some(key);
                self.revoking.take().unwrap().send(()).unwrap();
            }
            _ => panic!("T992 unrelated resource command"),
        }
    }
    fn confirm(&mut self) {
        let key = self.held.take().unwrap();
        if self
            .wire
            .multiplexer()
            .accepts_data(Channel::Speaker, key.generation)
        {
            self.wire
                .multiplexer()
                .close(Channel::Speaker, Kind::Close)
                .unwrap();
        }
        self.wire
            .multiplexer()
            .consumer_retired(Channel::Speaker, key.generation)
            .unwrap();
        self.management(MediaMessage::Result(key, Outcome::Revoked));
    }
    fn payload(&mut self, channel: Channel, length: u32) {
        let bytes = self.wire.payload().to_vec();
        if channel == Channel::Management {
            let mut offset = 0;
            while offset < bytes.len() {
                let (count, message) = self.decoder.feed(&bytes[offset..]).unwrap();
                offset += count;
                if let Some(message) = message {
                    self.command(message);
                }
            }
        } else {
            assert_eq!(channel, Channel::Speaker);
            if self.received.is_some() {
                self.bytes.extend_from_slice(&bytes);
                assert!(self.bytes.len() <= 8192);
                if self.bytes.len() >= 156 + 1972 {
                    self.received
                        .take()
                        .unwrap()
                        .send(self.bytes[..156 + 1972].to_vec())
                        .unwrap();
                    self.bytes.clear();
                }
            }
        }
        self.wire.multiplexer().release(channel, length).unwrap();
    }
    async fn run(mut self, mut stopped: oneshot::Receiver<()>) {
        loop {
            tokio::select! {
                _=&mut stopped=>return,
                result=&mut self.release,if self.held.is_some()=>{result.unwrap();self.confirm();}
                event=self.wire.next()=>{
                    if let WireEvent::Received{header,deliver}=event.unwrap() {
                        if deliver {self.payload(header.channel,header.length);}
                        self.wire.consume().unwrap();
                    }
                }
            }
        }
    }
}
#[tokio::test]
async fn t992_audio_private_tls_speaker_route_preserves_clock_pcm_and_delayed_remote_retirement() {
    let auth = super::tls::authenticated().await;
    let launch = Launch {
        executable: "/bin/false".into(),
        runtime: auth.fixture.server.settings.runtime.clone(),
        state: auth.fixture.server.settings.state.clone(),
        helper: auth.fixture.server.settings.helper.clone(),
    };
    let mut options = AudioOptions::new(Direction::Speakers);
    options.usb_peer = Some(auth.pin.as_str().into());
    options.profile.buffer_ms = 80;
    options.profile.background = true;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let local = tokio::net::TcpListener::from_std(listener.try_clone().unwrap()).unwrap();
    let token = "ab".repeat(32);
    let report = Report::new(watch::channel(Default::default()).0);
    let bridge = usb::native(&options, &token, listener, launch, &report).unwrap();
    let (server_stop, server_done) = oneshot::channel();
    let running = tokio::spawn(async move {
        let mut fixture = auth.fixture;
        tokio::select! {result=fixture.server.run()=>panic!("server failed {result:?}"),_=server_done=>()}
        fixture
    });
    let (received, pcm) = oneshot::channel();
    let (revoking, revoked) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let (peer_stop, peer_done) = oneshot::channel();
    let peer = Peer::new(
        auth.tablet,
        options.profile.invitation_profile().unwrap().1,
        received,
        revoking,
        released,
    );
    let tablet = tokio::spawn(peer.run(peer_done));
    let helper_root = tempfile::tempdir().unwrap();
    let helper = helper_root.path().join("audio-helper");
    std::fs::write(
        &helper,
        format!(
            "#!/usr/bin/python3\nimport pathlib,sys,time\n{}\n",
            audio::session_tests::PCM
        ),
    )
    .unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let session = audio::Session {
        listener: &local,
        connection: &bridge,
        helper: &helper,
        options: &options,
        token: &token,
    };
    let (stop, mut stopped) = watch::channel(false);
    let work = session.serve(&report, &mut stopped);
    tokio::pin!(work);
    let bytes = tokio::select! {result=&mut work=>panic!("premature audio {result:?}"),result=pcm=>result.unwrap()};
    assert_eq!(&bytes[..64], token.as_bytes());
    assert_eq!(&bytes[64..72], b"BLAUD002");
    assert_eq!(
        &bytes[144..156],
        &[2, 2, 0, 0, 187, 128, 1, 224, 2, 1, 0, 80]
    );
    assert_eq!(&bytes[156..164], &bytes[136..144]);
    assert_eq!(&bytes[182..184], &[2, 0]);
    assert_eq!(
        blent_config::audio::ClockSample::decode(&bytes[184..208]).unwrap(),
        Some(blent_config::audio::ClockSample {
            epoch: 3,
            frames: 96000,
            nanos: 2000000001
        })
    );
    assert_eq!(&bytes[208..], b"\xd2\x04\x1f\xef".repeat(480));
    stop.send_replace(true);
    bounded(&mut work).await.unwrap();
    assert!(!bridge.retired());
    let close = bridge.close();
    tokio::pin!(close);
    tokio::select! {result=&mut close=>panic!("premature remote proof {result:?}"),result=revoked=>result.unwrap()}
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
