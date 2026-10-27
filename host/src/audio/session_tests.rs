//! T992: original media authority joined to actual local helper ownership.
use super::*;
use crate::camera_registration::test_support::{authenticated, view, Rig};
use blent_config::{
    audio::{AudioProfile, ClockSample, Processing},
    connection::{control::State as LinkState, management::Profile, registration_client::Phase},
};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;

#[test]
fn t999_audio_microphone_usb_capability_tracks_native_accessory_build() {
    assert_eq!(
        transport_capabilities(Direction::Microphone).usb,
        cfg!(feature = "accessory-transport")
    );
}

struct Fixture {
    root: tempfile::TempDir,
    listener: TcpListener,
    options: AudioOptions,
    helper: PathBuf,
}
impl Fixture {
    async fn new(body: &str) -> Self {
        Self::new_direction(body, Direction::Speakers).await
    }
    async fn new_direction(body: &str, direction: Direction) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("helper");
        let source = format!(
            "#!/usr/bin/python3\nimport os,pathlib,sys,time\nroot=pathlib.Path(__file__).parent\n(root/'pid').write_text(str(os.getpid()))\n{body}\n"
        );
        std::fs::write(&helper, source).unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut options = AudioOptions::new(direction);
        options.usb_peer = Some(crate::display_test_support::peer(2).as_str().into());
        options.profile.buffer_ms = 80;
        options.profile.background = true;
        Self {
            root,
            helper,
            listener: TcpListener::bind("127.0.0.1:0").await.unwrap(),
            options,
        }
    }
    fn bridge(&self, rig: &Rig) -> crate::media_connection::Bridge<AudioProfile> {
        let (resource, audio) = self.options.profile.invitation_profile().unwrap();
        let request = crate::media_request::request(
            self.options.target().unwrap(),
            resource,
            Profile::Audio(audio),
            &"ab".repeat(32),
        )
        .unwrap();
        crate::media_connection::Bridge::owned(
            &self.options.profile,
            &"ab".repeat(32),
            rig.media(request).unwrap(),
        )
        .unwrap()
    }
    fn session<'a>(
        &'a self,
        connection: &'a dyn Connection<AudioProfile>,
        token: &'a str,
    ) -> Session<'a> {
        Session {
            listener: &self.listener,
            connection,
            helper: &self.helper,
            options: &self.options,
            token,
        }
    }
    async fn peer(&self) -> TcpStream {
        let mut socket = TcpStream::connect(self.listener.local_addr().unwrap())
            .await
            .unwrap();
        let magic: &[u8] = b"BLAUREQ2";
        socket
            .write_all(
                &[
                    magic,
                    "ab".repeat(32).as_bytes(),
                    &[7, 0, self.options.profile.direction as u8, 2],
                ]
                .concat(),
            )
            .await
            .unwrap();
        socket
    }
    fn assert_helper_reaped(&self) {
        let pid: i32 = std::fs::read_to_string(self.root.path().join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "T992 owned helper still live"
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}
pub(crate) const MICROPHONE_PCM: &str = "sys.stdout.buffer.write(b'READY\\n');sys.stdout.buffer.flush()\nsize=985\ndata=sys.stdin.buffer.read(size)\n(root/'capture').write_bytes(data)\nwhile len(sys.stdin.buffer.read(size))==size: pass";

pub(crate) fn microphone_packet(hello: &[u8], with_clock: bool, sequence: u64) -> Vec<u8> {
    assert_eq!(hello.len(), 156);
    assert_eq!(&hello[..64], "ab".repeat(32).as_bytes());
    assert_eq!(
        &hello[144..156],
        &[1, 1, 0, 0, 187, 128, 1, 224, 2, 1, 0, 80]
    );
    assert_eq!(&hello[64..72], b"BLAUD002");
    let mut packet = Vec::from(&hello[136..144]);
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(&(sequence + 1).to_be_bytes());
    packet.extend_from_slice(&984u16.to_be_bytes());
    packet.extend_from_slice(&[1, 0]);
    packet.extend_from_slice(
        &ClockSample::encode(with_clock.then_some(ClockSample {
            epoch: 3,
            frames: sequence * 480,
            nanos: 2000000001 + sequence * 10000000,
        }))
        .unwrap(),
    );
    packet.extend_from_slice(&1234i16.to_le_bytes().repeat(480));
    packet
}
pub(crate) async fn captured(root: &Path, with_clock: bool) {
    let bytes = bounded(async {
        loop {
            if let Ok(bytes) = std::fs::read(root.join("capture")) {
                if bytes.len() == 985 {
                    break bytes;
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert_eq!(bytes[0], 0);
    let offset = 25;
    assert_eq!(&bytes[offset..], &1234i16.to_le_bytes().repeat(480));
    assert_eq!(
        ClockSample::decode(&bytes[1..25]).unwrap(),
        with_clock.then_some(ClockSample {
            epoch: 3,
            frames: 0,
            nanos: 2000000001
        })
    );
}

#[tokio::test]
async fn t999_audio_microphone_clock_pcm_original_loss_and_native_retirement() {
    for with_clock in [false, true] {
        let rig = authenticated();
        let fixture = Fixture::new_direction(MICROPHONE_PCM, Direction::Microphone).await;
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (_stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            let mut socket = fixture.peer().await;
            let mut hello = [0; 156];
            socket.read_exact(&mut hello).await.unwrap();
            socket
                .write_all(&microphone_packet(&hello, with_clock, 0))
                .await
                .unwrap();
            captured(fixture.root.path(), with_clock).await;
            assert!(report.status().detail.contains("Effective Raw"));
            rig.client().set(Phase::Closing);
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        let (result, ()) =
            bounded(async { tokio::join!(session.serve(&report, &mut stopped), peer) }).await;
        assert!(result.is_err());
        fixture.assert_helper_reaped();
        assert!(report.native_retired());
        assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
        assert!(!bridge.retired());
        rig.complete();
        bridge.close().await.unwrap();
        assert!(bridge.retired());
    }
}

#[tokio::test]
async fn t1243_truncated_microphone_frames_retire_only_the_original_native_helper() {
    for with_clock in [false, true] {
        let final_byte = 1011;
        for length in [0, 1, 7, 8, 15, 23, 24, 25, 26, 27, 28, 29, 500, final_byte] {
            truncated_microphone(with_clock, length).await;
        }
    }
}

async fn truncated_microphone(with_clock: bool, length: usize) {
    let rig = authenticated();
    let fixture = Fixture::new_direction(MICROPHONE_PCM, Direction::Microphone).await;
    let bridge = fixture.bridge(&rig);
    let token = "ab".repeat(32);
    let session = fixture.session(&bridge, &token);
    let (_stop, mut stopped) = watch::channel(false);
    let report = Report::new(watch::channel(Default::default()).0);
    let peer = async {
        registered(&rig).await;
        let mut socket = fixture.peer().await;
        let mut hello = [0; 156];
        socket.read_exact(&mut hello).await.unwrap();
        socket
            .write_all(&microphone_packet(&hello, with_clock, 0))
            .await
            .unwrap();
        captured(fixture.root.path(), with_clock).await;
        let packet = microphone_packet(&hello, with_clock, 1);
        assert!(length < packet.len());
        socket.write_all(&packet[..length]).await.unwrap();
        socket.shutdown().await.unwrap();
    };
    let (result, ()) =
        bounded(async { tokio::join!(session.serve(&report, &mut stopped), peer) }).await;
    assert!(
        result.is_err(),
        "T1243 with_clock={with_clock} length={length}"
    );
    fixture.assert_helper_reaped();
    assert!(report.native_retired());
    assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
    assert!(!bridge.retired(), "T1243 native exit replaced remote proof");
    rig.complete();
    bridge.close().await.unwrap();
    assert!(bridge.retired());
}

async fn bounded<T>(work: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), work)
        .await
        .unwrap()
}
async fn registered(rig: &Rig) {
    while rig.calls.lock().unwrap().clients.is_empty() {
        tokio::task::yield_now().await;
    }
    rig.client().set(Phase::Active);
}
pub(crate) const PCM: &str = "sys.stdout.buffer.write(b'READY\\n');sys.stdout.buffer.flush()\nwhile True:\n    clock=(3).to_bytes(8,'big')+(96000).to_bytes(8,'big')+(2000000001).to_bytes(8,'big') if (root/'clock').exists() else bytes(24)\n    sys.stdout.buffer.write(b'\\x00'+clock+b'\\xd2\\x04\\x1f\\xef'*480);sys.stdout.buffer.flush();time.sleep(.01)";
async fn received(socket: &mut TcpStream, with_clock: bool) {
    let mut hello = [0; 156];
    socket.read_exact(&mut hello).await.unwrap();
    assert_eq!(&hello[..64], "ab".repeat(32).as_bytes());
    assert_eq!(&hello[64..72], b"BLAUD002");
    assert_eq!(&hello[144..], &[2, 2, 0, 0, 187, 128, 1, 224, 2, 1, 0, 80]);
    let mut frame = vec![0; 1972];
    socket.read_exact(&mut frame).await.unwrap();
    assert_eq!(&frame[..8], &hello[136..144]);
    assert_eq!(&frame[8..16], &0u64.to_be_bytes());
    assert_eq!(
        u16::from_be_bytes(frame[24..26].try_into().unwrap()) as usize,
        frame.len() - 28
    );
    assert_eq!(&frame[26..28], &[2, 0]);
    let offset = 52;
    let block = blent_config::audio::PcmBlock::from_le_bytes(Direction::Speakers, &frame[offset..])
        .unwrap();
    assert!(block
        .samples()
        .as_chunks::<2>()
        .0
        .iter()
        .all(|samples| *samples == [1234, -4321]));
    assert_eq!(
        ClockSample::decode(&frame[28..52]).unwrap(),
        with_clock.then_some(ClockSample {
            epoch: 3,
            frames: 96000,
            nanos: 2000000001
        })
    );
}
#[tokio::test]
async fn t992_audio_original_pending_stop_and_loss_do_not_start_helper() {
    for (direction, lost) in [
        (Direction::Speakers, false),
        (Direction::Microphone, false),
        (Direction::Speakers, true),
        (Direction::Microphone, true),
    ] {
        let rig = authenticated();
        let fixture = Fixture::new_direction(PCM, direction).await;
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            if lost {
                rig.source.status.lock().unwrap().view = Some(view(&[(1, 2, LinkState::Closing)]));
            } else {
                stop.send_replace(true);
            }
        };
        let (result, ()) =
            bounded(async { tokio::join!(session.serve(&report, &mut stopped), peer) }).await;
        assert_eq!(result.is_err(), lost);
        assert!(!fixture.root.path().join("pid").exists());
        assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
        assert!(!bridge.retired());
        rig.complete();
        bridge.close().await.unwrap();
        assert!(bridge.retired());
    }
}
#[tokio::test]
async fn t992_audio_clock_pcm_processing_override_and_original_loss_retire_only_this_session() {
    for with_clock in [false, true] {
        let rig = authenticated();
        let fixture = Fixture::new(PCM).await;
        if with_clock {
            std::fs::write(fixture.root.path().join("clock"), "").unwrap();
        }
        assert_eq!(fixture.options.profile.processing, Processing::Speech);
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (_stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            let mut socket = fixture.peer().await;
            received(&mut socket, with_clock).await;
            assert!(report.status().detail.contains("Effective Raw"));
            rig.client().set(Phase::Closing);
            tokio::time::sleep(Duration::from_millis(200)).await;
            drop(socket);
        };
        let (result, ()) =
            bounded(async { tokio::join!(session.serve(&report, &mut stopped), peer) }).await;
        assert!(result.is_err());
        fixture.assert_helper_reaped();
        assert_eq!(rig.calls.lock().unwrap().requests.len(), 1);
        assert!(!bridge.retired());
        rig.complete();
        bridge.close().await.unwrap();
        assert!(bridge.retired());
    }
}
#[tokio::test]
async fn t992_audio_native_failure_and_stall_leave_original_route_owned_until_actual_proof() {
    for (direction, body) in [Direction::Microphone, Direction::Speakers]
        .into_iter()
        .flat_map(|direction| {
            [
                "sys.stdout.buffer.write(b'WRONG!');sys.stdout.buffer.flush();time.sleep(30)",
                "sys.stdout.buffer.write(b'READY\\n');sys.stdout.buffer.flush();time.sleep(30)",
            ]
            .map(|body| (direction, body))
        })
    {
        let rig = authenticated();
        let fixture = Fixture::new_direction(body, direction).await;
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (_stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            let socket = fixture.peer().await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            drop(socket);
        };
        let (result, ()) =
            bounded(async { tokio::join!(session.serve(&report, &mut stopped), peer) }).await;
        assert!(result.is_err());
        fixture.assert_helper_reaped();
        assert!(!bridge.retired());
        rig.complete();
        bridge.close().await.unwrap();
        assert!(bridge.retired());
    }
}
struct Held {
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    entered: std::sync::mpsc::Sender<()>,
}
impl Drop for Held {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release.lock().unwrap().recv().unwrap();
    }
}
#[tokio::test]
async fn t992_audio_stuck_native_lease_never_reports_success_or_releases_actual_completion() {
    held_native(Direction::Speakers).await;
}
#[tokio::test]
async fn t999_audio_microphone_stuck_native_lease_retains_independent_completion() {
    held_native(Direction::Microphone).await;
}
async fn held_native(direction: Direction) {
    let rig = authenticated();
    let fixture = Fixture::new_direction(
        if direction == Direction::Speakers {
            PCM
        } else {
            MICROPHONE_PCM
        },
        direction,
    )
    .await;
    let bridge = fixture.bridge(&rig);
    let token = "ab".repeat(32);
    let session = fixture.session(&bridge, &token);
    let (stop, mut stopped) = watch::channel(false);
    let report = Report::new(watch::channel(Default::default()).0);
    let (release, held) = std::sync::mpsc::channel();
    let held = Mutex::new(Some(held));
    let (entered, entry) = std::sync::mpsc::channel();
    let completion = Arc::new(Mutex::new(None));
    let start = |path: &Path, profile: AudioProfile, token: &str| {
        let mut child = process::spawn(path, profile, token)?;
        *completion.lock().unwrap() = Some(child.completion());
        child.keep_alive(Held {
            release: Mutex::new(held.lock().unwrap().take().unwrap()),
            entered: entered.clone(),
        });
        Ok(child)
    };
    let peer = async {
        registered(&rig).await;
        let mut socket = fixture.peer().await;
        if direction == Direction::Speakers {
            received(&mut socket, false).await;
        } else {
            let mut hello = [0; 156];
            socket.read_exact(&mut hello).await.unwrap();
            socket
                .write_all(&microphone_packet(&hello, false, 0))
                .await
                .unwrap();
            captured(fixture.root.path(), false).await;
        }
        stop.send_replace(true);
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(socket);
    };
    let (result, ()) =
        bounded(async { tokio::join!(session.serve_with(&report, &mut stopped, &start), peer) })
            .await;
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("retirement incomplete"));
    entry.try_recv().unwrap();
    let proof = completion.lock().unwrap().clone().unwrap();
    assert!(!proof.is_complete());
    assert!(
        !report.native_retired(),
        "T995 helper proof lost at pipeline error"
    );
    assert!(!bridge.retired());
    release.send(()).unwrap();
    bounded(async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert!(report.native_retired());
    fixture.assert_helper_reaped();
    rig.complete();
    bridge.close().await.unwrap();
    assert!(bridge.retired());
}
#[test]
fn t992_audio_direction_capabilities_and_loss_errors_are_explicit() {
    assert_eq!(
        transport_capabilities(Direction::Microphone).usb,
        cfg!(feature = "accessory-transport")
    );
    assert_eq!(
        transport_capabilities(Direction::Speakers).usb,
        cfg!(feature = "accessory-transport")
    );
    assert!(lost(Ok(()))
        .unwrap_err()
        .to_string()
        .contains("original audio"));
    assert_eq!(
        lost(Err(anyhow::anyhow!("original fault")))
            .unwrap_err()
            .to_string(),
        "original fault"
    );
}
