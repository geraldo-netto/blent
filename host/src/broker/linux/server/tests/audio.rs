//! T992/T999: both audio directions over real private mutual TLS routing.
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
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{oneshot, watch};
mod duplex;

type AudioWire = Transport<Box<dyn crate::accessory::Stream>>;
struct AudioTrack {
    profile: blent_config::connection::management::Audio,
    progress: Arc<AtomicUsize>,
    channel: Channel,
    microphone: Option<Vec<u8>>,
    sequence: u64,
    key: Option<Key>,
    bytes: Vec<u8>,
    received: Option<oneshot::Sender<Vec<u8>>>,
    revoking: Option<oneshot::Sender<()>>,
    release: oneshot::Receiver<()>,
    held: Option<Key>,
}
impl AudioTrack {
    fn new(
        profile: blent_config::connection::management::Audio,
        direction: Direction,
        received: oneshot::Sender<Vec<u8>>,
        revoking: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            progress: Arc::new(AtomicUsize::new(0)),
            profile,
            channel: if direction == Direction::Microphone {
                Channel::Microphone
            } else {
                Channel::Speaker
            },
            microphone: None,
            sequence: 0,
            key: None,
            bytes: Vec::new(),
            received: Some(received),
            revoking: Some(revoking),
            release,
            held: None,
        }
    }
    fn command(&mut self, wire: &mut AudioWire, message: MediaMessage) {
        match message {
            MediaMessage::Offer(key, credential, Profile::Audio(profile)) => {
                assert_eq!(
                    key.resource,
                    if self.channel == Channel::Microphone {
                        Resource::Microphone
                    } else {
                        Resource::Speakers
                    }
                );
                assert_eq!(credential.as_bytes(), &[0xab; 32]);
                assert_eq!(profile, self.profile);
                self.key = Some(key);
                management(wire, MediaMessage::Prepared(key));
            }
            MediaMessage::Commit(key) => {
                assert_eq!(Some(key), self.key);
                wire.multiplexer()
                    .activate(self.channel, key.generation)
                    .unwrap();
                management(wire, MediaMessage::Result(key, Outcome::Admitted));
                let request = [
                    b"BLAUREQ2".as_slice(),
                    "ab".repeat(32).as_bytes(),
                    &[
                        7,
                        0,
                        if self.channel == Channel::Microphone {
                            1
                        } else {
                            2
                        },
                        2,
                    ],
                ]
                .concat();
                wire.multiplexer().offer(self.channel, request).unwrap();
            }
            MediaMessage::Revoke(key, _) => {
                assert_eq!(Some(key), self.key);
                self.held = Some(key);
                self.revoking.take().unwrap().send(()).unwrap();
            }
            _ => panic!("T992 unrelated resource command"),
        }
    }
    fn confirm(&mut self, wire: &mut AudioWire) {
        let key = self.held.take().unwrap();
        if wire
            .multiplexer()
            .accepts_data(self.channel, key.generation)
        {
            wire.multiplexer().close(self.channel, Kind::Close).unwrap();
        }
        wire.multiplexer()
            .consumer_retired(self.channel, key.generation)
            .unwrap();
        management(wire, MediaMessage::Result(key, Outcome::Revoked));
    }
    fn audio(&mut self, channel: Channel, bytes: &[u8]) {
        assert_eq!(channel, self.channel);
        self.progress.fetch_add(bytes.len(), Ordering::SeqCst);
        if self.received.is_none() {
            return;
        }
        self.bytes.extend_from_slice(bytes);
        assert!(self.bytes.len() <= 8192);
        let count = if channel == Channel::Microphone {
            156
        } else {
            156 + 1972
        };
        if self.bytes.len() < count {
            return;
        }
        let result = self.bytes[..count].to_vec();
        if channel == Channel::Microphone {
            self.microphone = Some(result.clone());
        }
        self.received.take().unwrap().send(result).unwrap();
        self.bytes.clear();
    }
    fn capture(&mut self, wire: &mut AudioWire) {
        if self.held.is_some() {
            return;
        }
        let Some(hello) = self.microphone.as_ref() else {
            return;
        };
        let packet = audio::session_tests::microphone_packet(hello, true, self.sequence);
        if wire
            .multiplexer()
            .offer(Channel::Microphone, packet)
            .is_ok()
        {
            self.sequence += 1;
            self.progress.fetch_add(960, Ordering::SeqCst);
        }
    }
}
fn management(wire: &mut AudioWire, message: MediaMessage) {
    let mut bytes = [0; blent_config::connection::management::MAX_RECORD];
    let count = message.encode(&mut bytes).unwrap();
    wire.multiplexer()
        .offer(Channel::Management, bytes[..count].to_vec())
        .unwrap();
}

async fn retirement(track: Option<&mut AudioTrack>) {
    match track {
        Some(track) if track.held.is_some() => (&mut track.release).await.unwrap(),
        _ => std::future::pending::<()>().await,
    }
}
struct Peer {
    wire: AudioWire,
    decoder: Decoder,
    microphone: Option<AudioTrack>,
    speakers: Option<AudioTrack>,
}
impl Peer {
    fn empty(stream: Box<dyn crate::accessory::Stream>) -> Self {
        Self {
            wire: Transport::new(
                stream,
                Multiplexer::new(1, 1, 4096, 64, 64).unwrap(),
                Duration::from_secs(2),
            )
            .unwrap(),
            decoder: Decoder::new(),
            microphone: None,
            speakers: None,
        }
    }
    fn new(
        stream: Box<dyn crate::accessory::Stream>,
        profile: blent_config::connection::management::Audio,
        direction: Direction,
        received: oneshot::Sender<Vec<u8>>,
        revoking: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    ) -> Self {
        let track = AudioTrack::new(profile, direction, received, revoking, release);
        let mut peer = Self::empty(stream);
        if direction == Direction::Microphone {
            peer.microphone = Some(track);
        } else {
            peer.speakers = Some(track);
        }
        peer
    }
    fn duplex(
        stream: Box<dyn crate::accessory::Stream>,
        microphone: AudioTrack,
        speakers: AudioTrack,
    ) -> Self {
        Self {
            microphone: Some(microphone),
            speakers: Some(speakers),
            ..Self::empty(stream)
        }
    }
    fn track(&mut self, resource: Resource) -> &mut AudioTrack {
        match resource {
            Resource::Microphone => self.microphone.as_mut().unwrap(),
            Resource::Speakers => self.speakers.as_mut().unwrap(),
            _ => panic!("T1000 unrelated resource"),
        }
    }
    fn command(&mut self, message: MediaMessage) {
        let resource = match &message {
            MediaMessage::Offer(key, _, _)
            | MediaMessage::Commit(key)
            | MediaMessage::Revoke(key, _) => key.resource,
            _ => panic!("T1000 unexpected command"),
        };
        match resource {
            Resource::Microphone => self
                .microphone
                .as_mut()
                .unwrap()
                .command(&mut self.wire, message),
            Resource::Speakers => self
                .speakers
                .as_mut()
                .unwrap()
                .command(&mut self.wire, message),
            _ => panic!("T1000 unrelated resource"),
        }
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
            self.audio(channel, &bytes);
        }
        self.wire.multiplexer().release(channel, length).unwrap();
    }

    fn audio(&mut self, channel: Channel, bytes: &[u8]) {
        self.track(if channel == Channel::Microphone {
            Resource::Microphone
        } else {
            Resource::Speakers
        })
        .audio(channel, bytes);
    }
    fn capture(&mut self) {
        if let Some(track) = &mut self.microphone {
            track.capture(&mut self.wire);
        }
    }
    async fn run(mut self, mut stopped: oneshot::Receiver<()>) {
        let mut capture = tokio::time::interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                _=&mut stopped=>return,
                _=capture.tick()=>self.capture(),
                ()=retirement(self.microphone.as_mut())=>self.microphone.as_mut().unwrap().confirm(&mut self.wire),
                ()=retirement(self.speakers.as_mut())=>self.speakers.as_mut().unwrap().confirm(&mut self.wire),
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
    private_tls_audio(Direction::Speakers).await;
}
#[tokio::test]
async fn t999_audio_private_tls_microphone_preserves_pcm_clock_and_delayed_remote_retirement() {
    private_tls_audio(Direction::Microphone).await;
}
async fn private_tls_audio(direction: Direction) {
    let auth = super::tls::authenticated().await;
    let launch = Launch {
        executable: "/bin/false".into(),
        runtime: auth.fixture.server.settings.runtime.clone(),
        state: auth.fixture.server.settings.state.clone(),
        helper: auth.fixture.server.settings.helper.clone(),
    };
    let mut options = AudioOptions::new(direction);
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
        direction,
        received,
        revoking,
        released,
    );
    let tablet = tokio::spawn(peer.run(peer_done));
    let helper_root = tempfile::tempdir().unwrap();
    let helper = helper_root.path().join("audio-helper");
    std::fs::write(helper_root.path().join("clock"), "").unwrap();
    std::fs::write(
        &helper,
        format!(
            "#!/usr/bin/python3\nimport pathlib,sys,time\nroot=pathlib.Path(__file__).parent\n{}\n",
            if direction == Direction::Microphone {
                audio::session_tests::MICROPHONE_PCM
            } else {
                audio::session_tests::PCM
            }
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
    if direction == Direction::Speakers {
        speaker_samples(&bytes, &token);
    } else {
        tokio::select! {result=&mut work=>panic!("premature native microphone {result:?}"),()=audio::session_tests::captured(helper_root.path(), true)=>()}
        assert_eq!(&bytes[..64], token.as_bytes());
        assert!(report.status().detail.contains("Effective Raw"));
    }
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

fn speaker_samples(bytes: &[u8], token: &str) {
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
}
