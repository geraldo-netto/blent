//! T992: original media authority joined to actual local helper ownership.
use super::*;
use crate::camera_registration::test_support::{authenticated, view, Rig};
use blent_config::{
    audio::{AudioProfile, ClockSample, Processing},
    connection::{control::State as LinkState, management::Profile, registration_client::Phase},
};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;

struct Fixture {
    root: tempfile::TempDir,
    listener: TcpListener,
    options: AudioOptions,
    helper: PathBuf,
}
impl Fixture {
    async fn new(body: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("helper");
        let source = format!("#!/usr/bin/python3\nimport os,pathlib,sys,time\nroot=pathlib.Path(__file__).parent\n(root/'pid').write_text(str(os.getpid()))\n{body}\n");
        std::fs::write(&helper, source).unwrap();
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut options = AudioOptions::new(Direction::Speakers);
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
    async fn peer(&self, clocked: bool) -> TcpStream {
        let mut socket = TcpStream::connect(self.listener.local_addr().unwrap())
            .await
            .unwrap();
        let magic: &[u8] = if clocked { b"BLAUREQ2" } else { b"BLAUREQ1" };
        socket
            .write_all(&[magic, "ab".repeat(32).as_bytes(), &[7, 0, 2, 2]].concat())
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
pub(crate) const PCM: &str = "sys.stdout.buffer.write(b'READY\\n');sys.stdout.buffer.flush()\nwhile True:\n    clock=(3).to_bytes(8,'big')+(96000).to_bytes(8,'big')+(2000000001).to_bytes(8,'big') if 'clocked' in sys.argv else b''\n    sys.stdout.buffer.write(b'\\x00'+clock+b'\\xd2\\x04\\x1f\\xef'*480);sys.stdout.buffer.flush();time.sleep(.01)";
async fn received(socket: &mut TcpStream, clocked: bool) {
    let mut hello = [0; 156];
    socket.read_exact(&mut hello).await.unwrap();
    assert_eq!(&hello[..64], "ab".repeat(32).as_bytes());
    assert_eq!(
        &hello[64..72],
        if clocked { b"BLAUD002" } else { b"BLAUD001" }
    );
    assert_eq!(&hello[144..], &[2, 2, 0, 0, 187, 128, 1, 224, 2, 1, 0, 80]);
    let mut frame = vec![0; if clocked { 1972 } else { 1948 }];
    socket.read_exact(&mut frame).await.unwrap();
    assert_eq!(&frame[..8], &hello[136..144]);
    assert_eq!(&frame[8..16], &0u64.to_be_bytes());
    assert_eq!(
        u16::from_be_bytes(frame[24..26].try_into().unwrap()) as usize,
        frame.len() - 28
    );
    assert_eq!(&frame[26..28], &[2, 0]);
    let offset = if clocked { 52 } else { 28 };
    let block = blent_config::audio::PcmBlock::from_le_bytes(Direction::Speakers, &frame[offset..])
        .unwrap();
    assert!(block
        .samples()
        .as_chunks::<2>()
        .0
        .iter()
        .all(|samples| *samples == [1234, -4321]));
    if clocked {
        assert_eq!(
            ClockSample::decode(&frame[28..52]).unwrap(),
            Some(ClockSample {
                epoch: 3,
                frames: 96000,
                nanos: 2000000001
            })
        );
    }
}
#[tokio::test]
async fn t992_audio_original_pending_stop_and_loss_do_not_start_helper() {
    for lost in [false, true] {
        let rig = authenticated();
        let fixture = Fixture::new(PCM).await;
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
    for clocked in [false, true] {
        let rig = authenticated();
        let fixture = Fixture::new(PCM).await;
        assert_eq!(fixture.options.profile.processing, Processing::Speech);
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (_stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            let mut socket = fixture.peer(clocked).await;
            received(&mut socket, clocked).await;
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
    for body in [
        "sys.stdout.buffer.write(b'WRONG!');sys.stdout.buffer.flush();time.sleep(30)",
        "sys.stdout.buffer.write(b'READY\\n');sys.stdout.buffer.flush();time.sleep(30)",
    ] {
        let rig = authenticated();
        let fixture = Fixture::new(body).await;
        let bridge = fixture.bridge(&rig);
        let token = "ab".repeat(32);
        let session = fixture.session(&bridge, &token);
        let (_stop, mut stopped) = watch::channel(false);
        let report = Report::new(watch::channel(Default::default()).0);
        let peer = async {
            registered(&rig).await;
            let socket = fixture.peer(false).await;
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
    let rig = authenticated();
    let fixture = Fixture::new(PCM).await;
    let bridge = fixture.bridge(&rig);
    let token = "ab".repeat(32);
    let session = fixture.session(&bridge, &token);
    let (stop, mut stopped) = watch::channel(false);
    let report = Report::new(watch::channel(Default::default()).0);
    let (release, held) = std::sync::mpsc::channel();
    let held = Mutex::new(Some(held));
    let (entered, entry) = std::sync::mpsc::channel();
    let completion = Arc::new(Mutex::new(None));
    let start = |path: &Path, profile: AudioProfile, token: &str, clocked: bool| {
        let mut child = process::spawn_with_clock(path, profile, token, clocked)?;
        *completion.lock().unwrap() = Some(child.completion());
        child.keep_alive(Held {
            release: Mutex::new(held.lock().unwrap().take().unwrap()),
            entered: entered.clone(),
        });
        Ok(child)
    };
    let peer = async {
        registered(&rig).await;
        let mut socket = fixture.peer(false).await;
        received(&mut socket, false).await;
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
    assert!(!transport_capabilities(Direction::Microphone).usb);
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
