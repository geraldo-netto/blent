//! T1000: independent native helpers and remote retirement on one TLS connection.
use super::*;

struct Signals {
    pcm: oneshot::Receiver<Vec<u8>>,
    revoked: oneshot::Receiver<()>,
    release: Option<oneshot::Sender<()>>,
    progress: Arc<AtomicUsize>,
}
impl Signals {
    fn new(options: &AudioOptions) -> (AudioTrack, Self) {
        let (received, pcm) = oneshot::channel();
        let (revoking, revoked) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let track = AudioTrack::new(
            options.profile.invitation_profile().unwrap().1,
            options.profile.direction,
            received,
            revoking,
            released,
        );
        let signals = Self {
            progress: track.progress.clone(),
            pcm,
            revoked,
            release: Some(release),
        };
        (track, signals)
    }
    async fn progresses(&self) {
        let before = self.progress.load(Ordering::SeqCst);
        bounded(async {
            while self.progress.load(Ordering::SeqCst) < before + 3840 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
    }
}

struct NativeAudio {
    options: AudioOptions,
    listener: tokio::net::TcpListener,
    bridge: usb::Bridge,
    report: Report,
    root: tempfile::TempDir,
    helper: std::path::PathBuf,
    token: String,
}
impl NativeAudio {
    fn new(direction: Direction, pin: &PeerId, launch: &Launch) -> Arc<Self> {
        let mut options = AudioOptions::new(direction);
        options.usb_peer = Some(pin.as_str().into());
        options.profile.buffer_ms = 80;
        options.profile.background = true;
        let native = TcpListener::bind("127.0.0.1:0").unwrap();
        native.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(native.try_clone().unwrap()).unwrap();
        let token = "ab".repeat(32);
        let report = Report::new(watch::channel(Default::default()).0);
        let bridge = usb::native(&options, &token, native, launch.clone(), &report).unwrap();
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("audio-helper");
        std::fs::write(root.path().join("clock"), "").unwrap();
        let body = match direction {
            Direction::Microphone => audio::session_tests::MICROPHONE_PCM,
            Direction::Speakers => audio::session_tests::PCM,
        };
        std::fs::write(&helper, format!(
            "#!/usr/bin/python3\nimport pathlib,sys,time\nroot=pathlib.Path(__file__).parent\n{body}\n"
        )).unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        Arc::new(Self {
            options,
            listener,
            bridge,
            report,
            root,
            helper,
            token,
        })
    }
    fn start(
        self: &Arc<Self>,
    ) -> (
        watch::Sender<bool>,
        tokio::task::JoinHandle<anyhow::Result<()>>,
    ) {
        let original = self.clone();
        let (stop, mut stopped) = watch::channel(false);
        let work = tokio::spawn(async move {
            audio::Session {
                listener: &original.listener,
                connection: &original.bridge,
                helper: &original.helper,
                options: &original.options,
                token: &original.token,
            }
            .serve(&original.report, &mut stopped)
            .await
        });
        (stop, work)
    }
    async fn retire(&self, signals: &mut Signals, sibling: &Signals, sibling_native: &Self) {
        assert!(
            !self.bridge.retired(),
            "T1000 native helper is not remote proof"
        );
        let close = self.bridge.close();
        tokio::pin!(close);
        tokio::select! {
            result=&mut close=>panic!("T1000 premature remote retirement {result:?}"),
            result=&mut signals.revoked=>result.unwrap(),
        }
        assert!(!self.report.native_retired());
        assert!(!sibling_native.bridge.retired());
        assert!(!sibling_native.report.native_retired());
        tokio::select! {
            result=&mut close=>panic!("T1000 held remote proof completed {result:?}"),
            ()=sibling.progresses()=>(),
        }
        signals.release.take().unwrap().send(()).unwrap();
        bounded(&mut close).await.unwrap();
        assert!(self.bridge.retired());
        assert!(
            !self.report.native_retired(),
            "T1000 original registration owner is still held"
        );
        sibling.progresses().await;
        assert!(!sibling_native.bridge.retired());
    }
}

#[tokio::test]
async fn t1000_audio_private_tls_duplex_independent_stop_and_held_remote_proof() {
    for microphone_first in [false, true] {
        duplex(microphone_first).await;
    }
}
async fn duplex(microphone_first: bool) {
    let auth = super::super::tls::authenticated().await;
    let launch = Launch {
        executable: "/bin/false".into(),
        runtime: auth.fixture.server.settings.runtime.clone(),
        state: auth.fixture.server.settings.state.clone(),
        helper: auth.fixture.server.settings.helper.clone(),
    };
    let microphone = NativeAudio::new(Direction::Microphone, &auth.pin, &launch);
    let speakers = NativeAudio::new(Direction::Speakers, &auth.pin, &launch);
    let (microphone_track, mut mic_signals) = Signals::new(&microphone.options);
    let (speaker_track, mut speaker_signals) = Signals::new(&speakers.options);
    let (server_stop, server_done) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut fixture = auth.fixture;
        tokio::select! {result=fixture.server.run()=>panic!("T1000 broker failed {result:?}"),_=server_done=>()}
        fixture
    });
    let (peer_stop, peer_done) = oneshot::channel();
    let tablet =
        tokio::spawn(Peer::duplex(auth.tablet, microphone_track, speaker_track).run(peer_done));
    let (mic_stop, mic_work) = microphone.start();
    let (speaker_stop, speaker_work) = speakers.start();
    let (mic, speaker) =
        bounded(async { tokio::join!(&mut mic_signals.pcm, &mut speaker_signals.pcm) }).await;
    assert_eq!(&mic.unwrap()[..64], microphone.token.as_bytes());
    speaker_samples(&speaker.unwrap(), &speakers.token);
    audio::session_tests::captured(microphone.root.path(), true).await;
    mic_signals.progresses().await;
    speaker_signals.progresses().await;
    stop_first(
        microphone_first,
        &microphone,
        &speakers,
        &mut mic_signals,
        &mut speaker_signals,
        mic_stop,
        mic_work,
        speaker_stop,
        speaker_work,
    )
    .await;
    let mic_report = microphone.report.clone();
    let speaker_report = speakers.report.clone();
    drop(microphone);
    drop(speakers);
    bounded(async {
        tokio::join!(mic_report.wait_native(), speaker_report.wait_native());
    })
    .await;
    assert!(mic_report.native_retired());
    assert!(speaker_report.native_retired());
    peer_stop.send(()).unwrap();
    bounded(tablet).await.unwrap();
    server_stop.send(()).unwrap();
    let mut fixture = bounded(server).await.unwrap();
    assert!(fixture.server.routes.is_empty());
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}
async fn stop_first(
    microphone_first: bool,
    microphone: &NativeAudio,
    speakers: &NativeAudio,
    mic_signals: &mut Signals,
    speaker_signals: &mut Signals,
    mic_stop: watch::Sender<bool>,
    mic_work: tokio::task::JoinHandle<anyhow::Result<()>>,
    speaker_stop: watch::Sender<bool>,
    speaker_work: tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let (
        first,
        second,
        first_signals,
        second_signals,
        first_stop,
        first_work,
        second_stop,
        second_work,
    ) = if microphone_first {
        (
            microphone,
            speakers,
            mic_signals,
            speaker_signals,
            mic_stop,
            mic_work,
            speaker_stop,
            speaker_work,
        )
    } else {
        (
            speakers,
            microphone,
            speaker_signals,
            mic_signals,
            speaker_stop,
            speaker_work,
            mic_stop,
            mic_work,
        )
    };
    first_stop.send_replace(true);
    bounded(first_work).await.unwrap().unwrap();
    assert!(!second_work.is_finished());
    first.retire(first_signals, second_signals, second).await;
    assert!(!second_work.is_finished());
    second_stop.send_replace(true);
    bounded(second_work).await.unwrap().unwrap();
    let close = second.bridge.close();
    tokio::pin!(close);
    tokio::select! {
        result=&mut close=>panic!("T1000 second remote proof completed {result:?}"),
        result=&mut second_signals.revoked=>result.unwrap(),
    }
    assert!(!second.report.native_retired());
    second_signals.release.take().unwrap().send(()).unwrap();
    bounded(&mut close).await.unwrap();
    assert!(
        !second.report.native_retired(),
        "T1000 original registration owner is still held"
    );
    assert!(second.bridge.retired());
}
