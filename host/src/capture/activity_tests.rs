//! T1015: source-to-native supervision in private harmless process fixtures.
use super::*;
use blent_config::host_activity::*;
use std::time::Duration;
use tokio::time::timeout;

pub(super) fn manager() -> CaptureManager {
    std::env::remove_var("DISPLAY");
    std::env::remove_var("WAYLAND_DISPLAY");
    std::env::set_var("XDG_CURRENT_DESKTOP", "T1015-fixture");
    let mut config = helper::activity_tests::configuration();
    config.width = 64;
    config.height = 64;
    // Absolute Python interpreter remains available; no compositor/FFmpeg tool
    // can be invoked by these native fixtures through the private executable path.
    std::env::set_var("PATH", config.helper_path.parent().unwrap());
    CaptureManager::new(config)
}
pub(super) fn settings(config: &CaptureConfig) -> EncoderSettings {
    EncoderSettings {
        encoder: config.encoder.clone(),
        fps: config.fps,
        bitrate: config.bitrate,
        width: config.width,
        height: config.height,
        quality: config.quality,
        width_mm: config.width_mm,
        height_mm: config.height_mm,
        stream_scale: config.stream_scale,
        geometry_ready: true,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}
#[tokio::test]
async fn t1015_pending_source_cannot_create_native_helper_or_fifo() {
    if ownership_tests::isolated(
        "capture::activity_tests::t1015_pending_source_cannot_create_native_helper_or_fifo",
    ) {
        return;
    }
    let mut manager = manager();
    let (_published, source) = watch::channel(Publication::Pending);
    manager.bind_display_work(Arc::new(blent::display_work::View::new(
        Some(source),
        None,
        false,
    )));
    let (_settings, settings_rx) = watch::channel(settings(&manager.config));
    let (_mode, display) = watch::channel(true);
    let (stop, stopped) = watch::channel(false);
    let (video, _receiver) = crate::video_queue::channel(8, Default::default());
    let fifo = fifo_path_for(manager.config.instance).unwrap();
    let task = tokio::spawn(async move {
        let result = manager
            .stream_frames(video.into(), settings_rx, display, stopped)
            .await;
        manager.shutdown().await;
        result
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let denied = !fifo.exists();
    stop.send_replace(true);
    timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        denied,
        "T1015 pending original source created native capture work"
    );
}

fn streaming_manager() -> CaptureManager {
    use std::os::unix::fs::PermissionsExt;
    let mut manager = manager();
    manager.config.height = 32;
    let path = manager.config.helper_path.clone();
    std::fs::write(
        &path,
        r#"#!/usr/bin/python3
import errno, os, pathlib, select, socket, sys
root = pathlib.Path(__file__).parent
args = sys.argv[1:]
fifo = args[args.index('--capture-fifo') + 1]
control = socket.socket(fileno=int(args[args.index('--capture-activity-fd') + 1]))
(root / 'helper.pid').write_text(str(os.getpid()))
while (root / 'hold-start').exists(): select.select([], [], [], 0.01)
print('EVDI_CONNECTED card4294967295', flush=True)
print('STREAM_SIZE 64 32', flush=True)
active = False
writer = None
while True:
    ready, _, _ = select.select([control], [], [], 0.01)
    if ready:
        request = control.recv(32)
        if not request: break
        received = request.hex()
        active = request[8] == 1
        if not active and writer is not None:
            os.close(writer)
            writer = None
        if (root / 'foreign-ack').exists(): request = request[:8] + bytes([1 - request[8]])
        if (root / 'hold-record').exists():
            (root / 'record-waiting').write_text('original request received; record held')
            while not (root / 'release-record').exists(): select.select([], [], [], 0.01)
        with (root / 'requests').open('a') as output: output.write(received + '\n')
        control.send(request)
    if not active: continue
    try:
        if writer is None: writer = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
        os.write(writer, bytes([80]) * (64 * 32 * 3 // 2))
    except OSError as error:
        if error.errno not in [errno.ENXIO, errno.EAGAIN, errno.EPIPE]: raise
        if error.errno == errno.EPIPE:
            os.close(writer)
            writer = None
"#,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    #[cfg(not(feature = "inproc-encoder"))]
    std::os::unix::fs::symlink("/usr/bin/ffmpeg", path.parent().unwrap().join("ffmpeg")).unwrap();
    manager
}
pub(super) fn capabilities() -> Capabilities {
    Capabilities {
        power: true,
        lock: true,
        display: true,
    }
}
pub(super) fn observe(feed: &mut Feed, revision: u64, locked: bool) {
    feed.observe(Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(locked),
            display_on: Some(true),
        },
    })
    .unwrap();
}
pub(super) fn publication(feed: &Feed) -> Publication {
    Publication::Observed {
        capabilities: capabilities(),
        snapshot: feed.snapshot(true, false),
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn requests() -> Vec<String> {
    std::fs::read_to_string(crate::runtime::runtime_dir().unwrap().join("requests"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}
async fn packet(video: &mut blent::video_queue::VideoReceiver) -> crate::media::VideoPacket {
    timeout(Duration::from_secs(5), video.recv())
        .await
        .unwrap()
        .unwrap()
}
#[tokio::test]
async fn t1015_actual_running_pause_retains_helper_and_resumes_fresh_codec_and_idr() {
    if ownership_tests::isolated("capture::activity_tests::t1015_actual_running_pause_retains_helper_and_resumes_fresh_codec_and_idr") { return; }
    let mut manager = streaming_manager();
    let headers = manager.codec_config();
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    let (published, source) = watch::channel(publication(&feed));
    let (video, mut received) = crate::video_queue::channel(8, Default::default());
    manager.bind_display_work(Arc::new(blent::display_work::View::new(
        Some(source),
        Some(video.viewer_activity()),
        true,
    )));
    let (_settings, settings_rx) = watch::channel(settings(&manager.config));
    let (_mode, display) = watch::channel(true);
    let (stop, stopped) = watch::channel(false);
    let task = tokio::spawn(async move {
        let result = manager
            .stream_frames(video.into(), settings_rx, display, stopped)
            .await;
        manager.shutdown().await;
        result
    });
    let first = packet(&mut received).await;
    assert!(first.is_idr);
    assert!(first.codec_config.is_some());
    let root = crate::runtime::runtime_dir().unwrap();
    let helper = std::fs::read_to_string(root.join("helper.pid")).unwrap();
    observe(&mut feed, 2, true);
    published.send_replace(publication(&feed));
    until(|| {
        requests()
            .last()
            .is_some_and(|record| record.ends_with("00"))
            && headers.current().is_none()
    })
    .await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        std::fs::read_to_string(root.join("helper.pid")).unwrap(),
        helper
    );
    while received.try_recv().is_ok() {}
    assert!(timeout(Duration::from_millis(30), received.recv())
        .await
        .is_err());
    observe(&mut feed, 3, false);
    published.send_replace(publication(&feed));
    let fresh = packet(&mut received).await;
    assert!(fresh.is_idr);
    assert!(fresh.codec_config.is_some());
    assert!(fresh.generation.load(std::sync::atomic::Ordering::Acquire));
    assert!(!Arc::ptr_eq(&first.generation, &fresh.generation));
    assert_ne!(first.seq, fresh.seq);
    assert_eq!(
        std::fs::read_to_string(root.join("helper.pid")).unwrap(),
        helper
    );
    stop.send_replace(true);
    timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

struct Stream {
    feed: Feed,
    published: Option<watch::Sender<Publication>>,
    settings: watch::Sender<EncoderSettings>,
    mode: watch::Sender<bool>,
    stop: watch::Sender<bool>,
    video: blent::video_queue::VideoSender,
    received: Option<blent::video_queue::VideoReceiver>,
    headers: CodecConfig,
    profile: crate::attachment::Attachment,
    task: Option<tokio::task::JoinHandle<Result<()>>>,
}
impl Stream {
    fn start(pause_without_viewer: bool) -> Self {
        use blent_config::connection::routes::{Identity, Link};
        let mut manager = streaming_manager();
        let headers = manager.codec_config();
        let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
        observe(&mut feed, 1, false);
        let (published, source) = watch::channel(publication(&feed));
        let (video, received) = crate::video_queue::channel(8, Default::default());
        manager.bind_display_work(Arc::new(blent::display_work::View::new(
            Some(source),
            Some(video.viewer_activity()),
            pause_without_viewer,
        )));
        let snapshot = settings(&manager.config);
        let (settings, settings_rx) = watch::channel(snapshot.clone());
        let profile = crate::attachment::Attachment::new(settings.clone());
        profile.begin_route(
            Some(Identity::observed("t1015-tablet".into(), Some("t1015-tablet".into())).unwrap()),
            Some(Link::Usb),
        );
        settings.send_replace(snapshot);
        manager.bind_profile(Arc::new(profile.clone()));
        let (mode, display) = watch::channel(true);
        let (stop, stopped) = watch::channel(false);
        let output = video.clone().into();
        let task = tokio::spawn(async move {
            let result = manager
                .stream_frames(output, settings_rx, display, stopped)
                .await;
            manager.shutdown().await;
            result
        });
        Self {
            feed,
            published: Some(published),
            settings,
            mode,
            stop,
            video,
            received: Some(received),
            headers,
            profile,
            task: Some(task),
        }
    }
    fn lock(&mut self, locked: bool) {
        let revision = self.feed.snapshot(true, false).demand.stamp.revision + 1;
        observe(&mut self.feed, revision, locked);
        self.published
            .as_ref()
            .unwrap()
            .send_replace(publication(&self.feed));
    }
    async fn packet(&mut self) -> crate::media::VideoPacket {
        packet(self.received.as_mut().unwrap()).await
    }
    async fn paused(&self) {
        until(|| {
            requests()
                .last()
                .is_some_and(|record| record.ends_with("00"))
                && self.headers.current().is_none()
        })
        .await;
    }
    fn helper(&self) -> String {
        std::fs::read_to_string(crate::runtime::runtime_dir().unwrap().join("helper.pid")).unwrap()
    }
    async fn fresh(&mut self, original: &crate::media::VideoPacket) -> crate::media::VideoPacket {
        let receiver = self.received.as_mut().unwrap();
        timeout(Duration::from_secs(5), async {
            loop {
                let packet = receiver.recv().await.unwrap();
                if !Arc::ptr_eq(&packet.generation, &original.generation) {
                    assert!(packet.is_idr);
                    assert!(packet.codec_config.is_some());
                    assert!(packet.generation.load(std::sync::atomic::Ordering::Acquire));
                    return packet;
                }
                assert!(
                    !packet.generation.load(std::sync::atomic::Ordering::Acquire),
                    "T1015 stale packet authority survived the fence"
                );
            }
        })
        .await
        .unwrap()
    }
    async fn stop(mut self) {
        self.stop.send_replace(true);
        timeout(Duration::from_secs(5), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
#[tokio::test]
async fn t1015_coalesced_lock_resume_still_quiesces_original_pixels_and_encoder() {
    if ownership_tests::isolated("capture::activity_tests::t1015_coalesced_lock_resume_still_quiesces_original_pixels_and_encoder") { return; }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let helper = stream.helper();
    stream.lock(true);
    stream.lock(false);
    let fresh = stream.fresh(&first).await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    assert_ne!(first.seq, fresh.seq);
    assert!(
        requests().iter().any(|record| record.ends_with("00")),
        "T1015 latest resume hid native pause"
    );
    assert_eq!(stream.helper(), helper);
    stream.stop().await;
}
#[tokio::test]
async fn t1015_same_identity_profile_handoff_retires_original_work_before_fresh_resume() {
    use blent_config::connection::routes::{Identity, Link};
    if ownership_tests::isolated("capture::activity_tests::t1015_same_identity_profile_handoff_retires_original_work_before_fresh_resume") { return; }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let helper = stream.helper();
    let snapshot = stream.settings.borrow().clone();
    stream.profile.begin_route(
        Some(Identity::observed("t1015-tablet".into(), Some("t1015-tablet".into())).unwrap()),
        Some(Link::Network),
    );
    stream.settings.send_replace(snapshot);
    stream.fresh(&first).await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(stream.helper(), helper);
    stream.stop().await;
}
#[tokio::test]
async fn t1015_last_viewer_pause_retains_monitor_and_resumes_fresh_generation() {
    if ownership_tests::isolated("capture::activity_tests::t1015_last_viewer_pause_retains_monitor_and_resumes_fresh_generation") { return; }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let helper = stream.helper();
    drop(stream.received.take());
    stream.paused().await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    stream.received = Some(stream.video.subscribe());
    stream.fresh(&first).await;
    assert_eq!(stream.helper(), helper);
    stream.stop().await;
}
#[tokio::test]
async fn t1015_disabled_no_viewer_preference_keeps_original_encoder_generation() {
    if ownership_tests::isolated("capture::activity_tests::t1015_disabled_no_viewer_preference_keeps_original_encoder_generation") { return; }
    let mut stream = Stream::start(false);
    let first = stream.packet().await;
    drop(stream.received.take());
    tokio::time::sleep(Duration::from_millis(30)).await;
    stream.received = Some(stream.video.subscribe());
    let same = stream.packet().await;
    assert!(Arc::ptr_eq(&first.generation, &same.generation));
    assert!(stream.headers.current().is_some());
    assert!(requests().iter().all(|record| record.ends_with("01")));
    stream.stop().await;
}
#[tokio::test]
async fn t1015_supported_source_closure_pauses_original_work() {
    if ownership_tests::isolated(
        "capture::activity_tests::t1015_supported_source_closure_pauses_original_work",
    ) {
        return;
    }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let helper = stream.helper();
    drop(stream.published.take());
    stream.paused().await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(stream.helper(), helper);
    stream.stop().await;
}

#[tokio::test]
async fn t1015_explicit_display_mode_detach_restarts_with_fresh_header_ownership() {
    if ownership_tests::isolated("capture::activity_tests::t1015_explicit_display_mode_detach_restarts_with_fresh_header_ownership") { return; }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    let header = stream.headers.current().unwrap();
    let helper = stream.helper();
    let pid: i32 = helper.parse().unwrap();
    stream.mode.send_replace(false);
    until(|| unsafe { libc::kill(pid, 0) } != 0).await;
    stream.mode.send_replace(true);
    stream.fresh(&first).await;
    assert_ne!(stream.helper(), helper);
    let current = stream.headers.current().unwrap();
    assert_ne!(
        header.as_ptr(),
        current.as_ptr(),
        "T1015 restarted generation retained the old codec header owner"
    );
    stream.stop().await;
}

#[tokio::test]
async fn t1015_supported_source_loss_failure_and_stop_cannot_restart_work() {
    if ownership_tests::isolated(
        "capture::activity_tests::t1015_supported_source_loss_failure_and_stop_cannot_restart_work",
    ) {
        return;
    }
    let mut stream = Stream::start(true);
    let first = stream.packet().await;
    stream.feed.lost();
    stream
        .published
        .as_ref()
        .unwrap()
        .send_replace(publication(&stream.feed));
    stream.paused().await;
    assert!(!first.generation.load(std::sync::atomic::Ordering::Acquire));
    let count = requests().len();
    for publication in [Publication::Failed, Publication::Stopped] {
        stream.published.as_ref().unwrap().send_replace(publication);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(stream.headers.current().is_none());
        assert_eq!(requests().len(), count);
    }
    stream.stop().await;
}
#[tokio::test]
async fn t1015_known_unsupported_source_retains_existing_stream_after_loss_and_closure() {
    if ownership_tests::isolated("capture::activity_tests::t1015_known_unsupported_source_retains_existing_stream_after_loss_and_closure") { return; }
    let mut stream = Stream::start(true);
    let mut feed = Feed::new(2, Capabilities::default(), Settings::default()).unwrap();
    feed.observe(Event {
        epoch: 2,
        revision: 1,
        state: HostState::default(),
    })
    .unwrap();
    stream
        .published
        .as_ref()
        .unwrap()
        .send_replace(Publication::Observed {
            capabilities: Capabilities::default(),
            snapshot: feed.snapshot(true, false),
        });
    let first = stream.packet().await;
    feed.lost();
    stream
        .published
        .as_ref()
        .unwrap()
        .send_replace(Publication::Observed {
            capabilities: Capabilities::default(),
            snapshot: feed.snapshot(true, false),
        });
    drop(stream.published.take());
    let next = stream.packet().await;
    assert!(Arc::ptr_eq(&first.generation, &next.generation));
    assert!(next.generation.load(std::sync::atomic::Ordering::Acquire));
    assert!(requests().iter().all(|record| record.ends_with("01")));
    stream.stop().await;
}
#[tokio::test]
async fn t1015_lock_during_actual_helper_handshake_retires_original_startup_before_resume() {
    if ownership_tests::isolated("capture::activity_tests::t1015_lock_during_actual_helper_handshake_retires_original_startup_before_resume") { return; }
    let mut stream = Stream::start(true);
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("hold-start"), "hold").unwrap();
    until(|| root.join("helper.pid").exists()).await;
    let helper = stream.helper();
    let pid: i32 = helper.parse().unwrap();
    stream.lock(true);
    until(|| unsafe { libc::kill(pid, 0) } != 0).await;
    assert!(stream.headers.current().is_none());
    assert!(
        requests().is_empty(),
        "T1015 incomplete original startup acquired pixels"
    );
    std::fs::remove_file(root.join("hold-start")).unwrap();
    stream.lock(false);
    let fresh = stream.packet().await;
    assert!(fresh.is_idr);
    assert!(fresh.codec_config.is_some());
    assert_ne!(stream.helper(), helper);
    stream.stop().await;
}
#[tokio::test]
async fn t1015_bad_resume_ack_retires_helper_and_uses_bounded_recovery_backoff() {
    if ownership_tests::isolated("capture::activity_tests::t1015_bad_resume_ack_retires_helper_and_uses_bounded_recovery_backoff") { return; }
    let stream = Stream::start(true);
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("foreign-ack"), "foreign").unwrap();
    until(|| root.join("helper.pid").exists()).await;
    let helper = stream.helper();
    let pid: i32 = helper.parse().unwrap();
    until(|| unsafe { libc::kill(pid, 0) } != 0).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        stream.helper(),
        helper,
        "T1015 failed resume spun native helper replacements"
    );
    assert_eq!(requests().len(), 1);
    assert!(stream.headers.current().is_none());
    stream.stop().await;
}

#[tokio::test]
async fn t1015_current_guard_denies_first_effect_and_obsolete_ready_profile_result() {
    use blent_config::connection::routes::{Identity, Link};
    let (_mode, display) = watch::channel(true);
    let (_stop, stopped) = watch::channel(false);
    let (settings, _updates) = watch::channel(settings(&CaptureConfig::default()));
    let profile = crate::attachment::Attachment::new(settings);
    let sources = activity::Sources::new(None, Some(Arc::new(profile.clone())));
    let authority = sources.begin(&display, &stopped).unwrap();
    let effects = std::sync::atomic::AtomicUsize::new(0);
    profile.begin_route(None, None);
    let result = authority
        .guarded(async {
            effects.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        })
        .await;
    assert!(result.is_none());
    assert_eq!(effects.load(std::sync::atomic::Ordering::Acquire), 0);
    let authority = sources.begin(&display, &stopped).unwrap();
    let result = authority
        .guarded(async {
            profile.begin_route(
                Some(Identity::observed("t1015-peer".into(), None).unwrap()),
                Some(Link::Usb),
            );
            9
        })
        .await;
    assert_eq!(
        result, None,
        "T1015 ready obsolete profile result escaped current-work guard"
    );
}

#[path = "registered_tests.rs"]
mod registered;

#[tokio::test]
async fn t1054_original_request_record_precedes_ack_even_when_recording_is_held() {
    if ownership_tests::isolated("capture::activity_tests::t1054_original_request_record_precedes_ack_even_when_recording_is_held") { return; }
    let mut manager = streaming_manager();
    manager.bind_display_work(Arc::new(blent::display_work::View::new(None, None, false)));
    manager.start_helper().await.unwrap();
    let child = manager.helper.child.as_ref().unwrap().completion();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(root.join("hold-record"), b"hold original request trace").unwrap();
    let acknowledged = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = acknowledged.clone();
    let (release, mut cleanup) = watch::channel(false);
    let native = blent::owned_task::Task::spawn(async move {
        let result = manager.helper.set_capture_active(true).await;
        observed.store(result.is_ok(), std::sync::atomic::Ordering::Release);
        let _ = cleanup.wait_for(|done| *done).await;
        manager.shutdown().await;
        while manager.helper.restart_ready().is_err() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        result.map(|_| ())
    })
    .unwrap();
    until(|| root.join("record-waiting").exists()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let published_early = acknowledged.load(std::sync::atomic::Ordering::Acquire);
    let recorded_early = requests();
    std::fs::write(root.join("release-record"), b"release original trace").unwrap();
    release.send_replace(true);
    native.await.unwrap().unwrap();
    until(|| child.is_complete()).await;
    assert!(
        !published_early,
        "T1054 ACK preceded original request trace"
    );
    assert!(recorded_early.is_empty());
    assert_eq!(requests().len(), 1);
}

#[tokio::test]
async fn t1054_forged_ack_cannot_replace_original_request_identity_in_trace() {
    if ownership_tests::isolated("capture::activity_tests::t1054_forged_ack_cannot_replace_original_request_identity_in_trace") { return; }
    let mut manager = streaming_manager();
    manager.bind_display_work(Arc::new(blent::display_work::View::new(None, None, false)));
    manager.start_helper().await.unwrap();
    let root = crate::runtime::runtime_dir().unwrap();
    std::fs::write(
        root.join("foreign-ack"),
        b"forge reply; retain original request",
    )
    .unwrap();
    let result = manager.helper.set_capture_active(true).await;
    manager.shutdown().await;
    while manager.helper.restart_ready().is_err() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(result.is_err());
    let original = requests();
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].len(), 18);
    assert!(
        original[0].ends_with("01"),
        "T1054 forged reply overwrote original request trace"
    );
}
