use super::lease_tests::private_root;
use super::*;
use blent_config::negotiation::{DecoderChoice, Profile, StreamProfile};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

async fn launch(lease: Lease, evidence: Arc<EncoderEvidence>, tracker: LatencyTracker) -> Handle {
    launch_with(
        lease,
        evidence,
        tracker,
        Arc::new(AtomicU64::new(0)),
        &Default::default(),
    )
    .await
    .unwrap()
}
async fn launch_with(
    lease: Lease,
    evidence: Arc<EncoderEvidence>,
    tracker: LatencyTracker,
    viewers: Arc<dyn blent::capture_contract::ViewerActivity>,
    scope: &blent::native_scope::Scope,
) -> Result<Handle> {
    let stop = lease.stop.clone();
    finish_startup(spawn(
        Mode::Adaptive,
        stop,
        move || Ok(lease),
        evidence,
        tracker,
        subjects::Subjects::new(Mode::Adaptive, viewers),
        scope,
    ))
    .await
}

pub(super) fn evidence(tracker: &LatencyTracker) -> Arc<EncoderEvidence> {
    tracker.encoder_started_with_decoder(
        "libx264",
        (640, 480, 30, 20000, 18),
        Some(DecoderChoice {
            name: "fixture".into(),
            stream: StreamProfile {
                codec: "h264".into(),
                format: Profile {
                    profile: "baseline".into(),
                    level: 31,
                    depth: 8,
                },
            },
            low_latency: false,
            operating_rate: None,
        }),
    )
}

pub(super) fn lease(directory: &Path) -> Lease {
    let fifo = directory.join("capture.nv12");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    Lease::open(&fifo, Default::default()).unwrap()
}

pub(super) async fn native_until(completed: impl FnMut() -> bool) {
    observe_native(Duration::from_secs(3), completed).await;
}

async fn observe_native(budget: Duration, mut completed: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + budget;
    while !completed() {
        assert!(
            std::time::Instant::now() < deadline,
            "Native observation did not finish within {budget:?}"
        );
        tokio::task::yield_now().await;
    }
}

async fn ack(
    handle: &Handle,
    tracker: &LatencyTracker,
    evidence: &Arc<EncoderEvidence>,
    seq: u32,
    pts: i64,
) {
    handle.note(seq, Some(pts), seq.is_multiple_of(2));
    tracker.on_encoded_for(seq, evidence);
    tokio::time::advance(Duration::from_millis(10)).await;
    tracker.on_rendered_from(
        seq,
        100,
        Some(&evidence.decoder.as_ref().unwrap().receipt()),
    );
    native_until(|| {
        !handle
            .timings
            .lock()
            .unwrap()
            .iter()
            .any(|frame| frame.sequence == seq)
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn t492_controller_trials_current_receipts_then_restores_cadence_on_stall() {
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let activity = tracker.encoder_activity(evidence.clone());
    let handle = launch(lease(dir.path()), evidence.clone(), tracker.clone()).await;
    for seq in 0..=16 {
        ack(&handle, &tracker, &evidence, seq, i64::from(seq) * 200_000).await;
        tokio::time::advance(Duration::from_millis(190)).await;
    }
    native_until(|| handle._lease.path.exists()).await;
    assert!(
        handle._lease.path.exists(),
        "T492: measured baseline must start a sparse trial"
    );
    for seq in 17..=32 {
        tokio::time::advance(Duration::from_millis(300)).await;
        ack(
            &handle,
            &tracker,
            &evidence,
            seq,
            3_200_000 + i64::from(seq - 16) * 500_000,
        )
        .await;
        tokio::time::advance(Duration::from_millis(190)).await;
    }
    assert!(handle._lease.path.exists());
    tokio::time::advance(Duration::from_secs(2)).await;
    native_until(|| !handle._lease.path.exists()).await;
    assert!(
        !handle._lease.path.exists(),
        "T492: no ACK progress must restore five updates/s"
    );
    drop(activity);
}

#[tokio::test(start_paused = true)]
async fn t492_retired_stream_and_cancellation_cannot_publish_into_successor() {
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let old = evidence(&tracker);
    let activity = tracker.encoder_activity(old.clone());
    let handle = launch(lease(dir.path()), old, tracker.clone()).await;
    handle._lease.publish(500).unwrap();
    let next = Arc::new(lease(dir.path()));
    next.publish(500).unwrap();
    let retained = handle._lease.clone();
    drop(handle);
    assert!(retained.publish(500).is_err());
    assert!(next
        .contents()
        .unwrap()
        .contains(&format!(" {} ", next.token)));
    drop(activity);
    let current = evidence(&tracker);
    let activity = tracker.encoder_activity(current.clone());
    let handle = launch(lease(dir.path()), current, tracker).await;
    tokio::task::yield_now().await;
    drop(activity);
    native_until(|| handle.task.is_finished()).await;
    assert!(handle.task.is_finished());
    next.clear();
    assert!(!next.path.exists());
}

#[tokio::test(start_paused = true)]
async fn t492_unknown_decoder_disabled_setting_and_invalid_fifo_never_start() {
    let tracker = LatencyTracker::new();
    let known = evidence(&tracker);
    let mut config = CaptureConfig::default();
    assert!(start(
        &config,
        known.clone(),
        tracker.clone(),
        Arc::new(AtomicU64::new(0)),
        &Default::default(),
    )
    .await
    .is_none());
    config.adaptive_idle = true;
    let unknown = tracker.encoder_started("libx264", known.format);
    assert!(start(
        &config,
        unknown,
        tracker.clone(),
        Arc::new(AtomicU64::new(0)),
        &Default::default(),
    )
    .await
    .is_none());
    config.instance = u32::MAX;
    assert!(start(
        &config,
        known,
        tracker,
        Arc::new(AtomicU64::new(0)),
        &Default::default()
    )
    .await
    .is_none());
    let dir = private_root();
    assert!(Lease::open(&dir.path().join("absent"), Default::default()).is_err());
    assert!(Lease::open(dir.path(), Default::default()).is_err());
}

#[tokio::test(start_paused = true)]
async fn t492_late_join_missing_timestamps_and_bounded_history_fail_closed() {
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let handle = launch(lease(dir.path()), evidence.clone(), tracker.clone()).await;
    let origin = Instant::now();
    for seq in 0..300 {
        handle.note(seq, Some(i64::from(seq)), false);
    }
    assert_eq!(handle.timings.lock().unwrap().len(), MAX_TIMINGS);
    assert!(take(&handle.timings, 0).is_none());
    assert_eq!(take(&handle.timings, 298).unwrap().sequence, 298);
    assert_eq!(handle.timings.lock().unwrap().len(), 1);
    handle.note(300, None, true);
    tracker.on_encoded_for(300, &evidence);
    tracker.on_rendered_from(300, 0, Some(&evidence.decoder.as_ref().unwrap().receipt()));
    let mut policy = Policy::new(Mode::Adaptive, false, true);
    consume(
        &mut policy,
        Conditions::default(),
        &handle.timings,
        &evidence,
        &mut 0,
        origin,
    );
    assert_eq!(policy.phase(), Phase::Rejected);
    for seq in 301..600 {
        tracker.on_encoded_for(seq, &evidence);
        tracker.on_rendered_from(seq, 0, Some(&evidence.decoder.as_ref().unwrap().receipt()));
    }
    let mut policy = Policy::new(Mode::Adaptive, false, true);
    consume(
        &mut policy,
        Conditions::default(),
        &handle.timings,
        &evidence,
        &mut 0,
        origin,
    );
    assert_eq!(
        policy.phase(),
        Phase::Rejected,
        "T492: discarded ACK history cannot certify a late join"
    );
    assert_eq!(micros(origin, origin - Duration::from_secs(1)), 0);
}

#[test]
fn t492_lease_files_are_private_bounded_expiring_and_inode_scoped() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = private_root();
    let first = lease(dir.path());
    first.publish(500).unwrap();
    assert_eq!(
        std::fs::metadata(&first.path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let fields: Vec<u64> = first
        .contents()
        .unwrap()
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    assert_eq!(
        &fields[..3],
        &[first.identity.0, first.identity.1, first.token]
    );
    assert_eq!(fields[4], 500);
    assert!(first.publish(1).is_err());
    first.publish(200).unwrap();
    assert!(!first.path.exists());
    std::fs::write(&first.path, vec![b'x'; 1000]).unwrap();
    assert_eq!(first.contents().unwrap().len(), 160);
    first.clear();
    assert!(first.path.exists(), "do not delete another owner's request");
    std::fs::remove_file(&first.path).unwrap();
    symlink(dir.path().join("absent"), &first.path).unwrap();
    assert!(first.contents().is_err());
    first.publish(500).unwrap();
    assert!(!first.path.is_symlink());
    std::fs::remove_file(&first.path).unwrap();
    std::fs::create_dir(&first.path).unwrap();
    assert!(first.publish(500).is_err());
}

#[test]
fn t492_helper_and_encoder_share_explicit_idle_option() {
    let dir = private_root();
    for enabled in [false, true] {
        let config = CaptureConfig {
            adaptive_idle: enabled,
            ..Default::default()
        };
        let helper = super::super::helper::HelperProcess::new();
        let command = helper
            .command(&config, &dir.path().join("capture.nv12"))
            .unwrap();
        let args: Vec<_> = command.as_std().get_args().collect();
        assert_eq!(
            args.contains(&std::ffi::OsStr::new("--idle-control-file")),
            enabled
        );
        let encoder = super::super::cli_encoder::CliEncoder { config: &config };
        let command = encoder.encoder_command(64, 64, false).unwrap();
        let args: Vec<_> = command.as_std().get_args().collect();
        let expression = args
            .windows(2)
            .find(|pair| pair[0] == "-force_key_frames")
            .unwrap()[1];
        assert_eq!(expression.to_string_lossy().contains("+0.9"), enabled);
    }
}

#[tokio::test(start_paused = true)]
async fn t492_control_publish_error_stops_the_controller() {
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let handle = launch(lease(dir.path()), evidence.clone(), tracker.clone()).await;
    std::fs::create_dir(&handle._lease.path).unwrap();
    for seq in 0..=16 {
        ack(&handle, &tracker, &evidence, seq, i64::from(seq) * 200_000).await;
        tokio::time::advance(Duration::from_millis(190)).await;
    }
    native_until(|| handle.task.is_finished()).await;
    assert!(handle.task.is_finished());
}

#[tokio::test]
async fn t492_completed_packets_retain_their_own_timing_before_broadcast() {
    use crate::media::Codec;
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let handle = launch(lease(dir.path()), evidence.clone(), tracker.clone()).await;
    let timings = handle.timings.clone();
    let pictures = vec![
        vec![
            0, 0, 1, 0x67, 0x11, 0, 0, 1, 0x68, 0x22, 0, 0, 1, 0x65, 0x80,
        ],
        vec![0, 0, 1, 0x41, 0x80],
    ];
    let stream = crate::framed_annex_b::tests::stream(Codec::H264, &pictures);
    let (tx, mut rx) = crate::video_queue::channel(8, Arc::new(AtomicBool::new(false)));
    super::super::cli_encoder::read_loop_with_idle(
        stream.as_slice(),
        tx.into(),
        Default::default(),
        tracker,
        Codec::H264,
        evidence.clone(),
        Some(handle),
    )
    .await
    .unwrap();
    for pts in 0..2 {
        let packet = rx.recv().await.unwrap();
        let timing = take(&timings, packet.seq).unwrap();
        assert_eq!(timing.pts_us, Some(pts));
        assert_eq!(timing.keyframe, pts == 0);
    }
    assert!(
        !evidence.active(),
        "T492: EOF retires both stream and idle controller"
    );
}

#[tokio::test(start_paused = true)]
async fn t492_new_viewer_invalidates_sparse_admission_even_without_missing_frames() {
    let dir = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let (tx, _initial) = crate::video_queue::channel(8, Arc::new(AtomicBool::new(false)));
    let handle = launch_with(
        lease(dir.path()),
        evidence,
        tracker,
        tx.viewer_epoch(),
        &Default::default(),
    )
    .await
    .unwrap();
    tokio::task::yield_now().await;
    handle._lease.publish(500).unwrap();
    let _new_viewer = tx.subscribe();
    tokio::time::advance(Duration::from_millis(250)).await;
    native_until(|| handle.task.is_finished()).await;
    assert!(!handle._lease.path.exists());
    assert!(
        handle.task.is_finished(),
        "T492: reconnect requires fresh encoder admission"
    );
}

struct HeldViewer {
    calls: std::sync::atomic::AtomicUsize,
    entered: std::sync::mpsc::Sender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl blent::capture_contract::ViewerActivity for HeldViewer {
    fn epoch(&self) -> u64 {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
        }
        0
    }
}
struct ReleaseViewer(std::sync::mpsc::Sender<()>);
impl Drop for ReleaseViewer {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t1029_aborted_idle_controller_retains_original_task_cleanup() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let _activity = tracker.encoder_activity(evidence.clone());
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = ReleaseViewer(release);
    let viewers = Arc::new(HeldViewer {
        calls: std::sync::atomic::AtomicUsize::new(0),
        entered: entered_tx,
        release: Mutex::new(held),
    });
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let handle = launch_with(lease(directory.path()), evidence, tracker, viewers, &scope)
        .await
        .unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(
        !handle.task.is_finished(),
        "T1029 original task must remain held"
    );
    drop(scope);
    drop(handle);
    let retained = !retired.is_complete();
    drop(release);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !retired.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        retained,
        "T1029 idle controller lost original task retirement"
    );
}

#[tokio::test]
async fn t1029_start_reports_original_cleanup_before_exposing_handle() {
    if crate::capture::ownership_tests::isolated(
        "capture::idle::tests::t1029_start_reports_original_cleanup_before_exposing_handle",
    ) {
        return;
    }
    let config = CaptureConfig {
        adaptive_idle: true,
        ..Default::default()
    };
    super::super::helper::ensure_fifo(&super::super::fifo_path_for(config.instance).unwrap())
        .unwrap();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let handle = start(
        &config,
        evidence,
        tracker,
        Arc::new(AtomicU64::new(0)),
        &scope,
    )
    .await
    .unwrap();
    let original = handle.completion();
    drop(scope);
    assert!(!retired.is_complete());
    drop(handle);
    tokio::time::timeout(Duration::from_secs(5), async {
        while !original.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(retired.is_complete());
}

#[tokio::test]
async fn t1029_capacity_exhaustion_preserves_compatible_cadence_without_worker() {
    if crate::capture::ownership_tests::isolated("capture::idle::tests::t1029_capacity_exhaustion_preserves_compatible_cadence_without_worker") { return; }
    std::env::set_var("BLENT_TASK_LIMIT", "1");
    let mut other = blent::owned_task::Task::spawn(std::future::pending()).unwrap();
    let config = CaptureConfig {
        adaptive_idle: true,
        ..Default::default()
    };
    let fifo = super::super::fifo_path_for(config.instance).unwrap();
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let (scope, retired) = blent::native_scope::Scope::default().group();
    assert!(start(
        &config,
        evidence,
        tracker,
        Arc::new(AtomicU64::new(0)),
        &scope
    )
    .await
    .is_none());
    assert!(!fifo.with_extension("idle").exists());
    drop(scope);
    assert!(retired.is_complete());
    assert!(!other.is_finished());
    other.abort();
    assert!((&mut other).await.unwrap_err().is_cancelled());
    assert!(other.is_finished());
}

fn held_files() -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    hold_files(|held| {
        let _ = held.recv_timeout(Duration::from_millis(400));
    })
}

fn hold_files(
    wait: impl FnOnce(std::sync::mpsc::Receiver<()>) + Send + 'static,
) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _files = LEASE_FILES.lock().unwrap();
        entered_tx.send(()).unwrap();
        wait(held);
    });
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    (release, worker)
}
pub(super) async fn idle_completed(original: &blent_config::retirement::Completion) {
    observe_native(Duration::from_secs(5), || original.is_complete()).await;
}

#[tokio::test]
async fn t1034_idle_drop_revokes_without_waiting_for_native_file_lock() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let _activity = tracker.encoder_activity(evidence.clone());
    let handle = launch(lease(directory.path()), evidence, tracker).await;
    let original = handle.completion();
    let (release, worker) = held_files();
    let start = std::time::Instant::now();
    drop(handle);
    let elapsed = start.elapsed();
    let retained = !original.is_complete();
    let _ = release.send(());
    worker.join().unwrap();
    idle_completed(&original).await;
    assert!(retained, "T1034 lock wait cannot grant native cleanup");
    assert!(
        elapsed < Duration::from_millis(100),
        "T1034 idle drop blocked control for {elapsed:?}"
    );
}
#[tokio::test]
async fn t1034_idle_file_effects_do_not_block_control_runtime() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let _activity = tracker.encoder_activity(evidence.clone());
    let native = lease(directory.path());
    let (release, worker) = held_files();
    let handle = launch(native, evidence, tracker).await;
    let original = handle.completion();
    let start = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(10)).await;
    let elapsed = start.elapsed();
    drop(handle);
    let _ = release.send(());
    worker.join().unwrap();
    idle_completed(&original).await;
    assert!(
        elapsed < Duration::from_millis(100),
        "T1034 native file effect blocked runtime for {elapsed:?}"
    );
}

#[tokio::test]
async fn t1034_unobserved_completed_startup_retains_queued_lease_cleanup() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    drop(tracker.encoder_activity(evidence.clone()));
    let native = lease(directory.path());
    let stop = native.stop.clone();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let startup = spawn(
        Mode::Adaptive,
        stop,
        move || Ok(native),
        evidence,
        tracker,
        subjects::Subjects::new(Mode::Adaptive, Arc::new(AtomicU64::new(0))),
        &scope,
    )
    .unwrap();
    let original = startup.task.as_ref().unwrap().completion();
    native_until(|| startup.task.as_ref().unwrap().is_finished()).await;
    drop(scope);
    let (release, worker) = held_files();
    let start = std::time::Instant::now();
    drop(startup);
    let elapsed = start.elapsed();
    let retained = !original.is_complete();
    let _ = release.send(());
    worker.join().unwrap();
    idle_completed(&original).await;
    assert!(retired.is_complete());
    assert!(retained, "T1034 queued startup lost original lease cleanup");
    assert!(
        elapsed < Duration::from_millis(100),
        "T1034 queued startup cleanup blocked caller for {elapsed:?}"
    );
}

#[tokio::test]
async fn t1034_canceled_startup_keeps_late_native_lease_creation() {
    let directory = private_root();
    let fifo = directory.path().join("late.nv12");
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let stop = Arc::new(worker::Stop::default());
    let creating = stop.clone();
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = ReleaseViewer(release);
    let (created_tx, created) = tokio::sync::oneshot::channel();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let startup = spawn(
        Mode::Adaptive,
        stop,
        move || {
            entered_tx.send(()).unwrap();
            held.recv().unwrap();
            let lease = Lease::open(&fifo, creating)?;
            created_tx.send(()).unwrap();
            Ok(lease)
        },
        evidence,
        tracker,
        subjects::Subjects::new(Mode::Adaptive, Arc::new(AtomicU64::new(0))),
        &scope,
    )
    .unwrap();
    let original = startup.task.as_ref().unwrap().completion();
    let parent = tokio::spawn(startup.finish());
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    parent.abort();
    assert!(matches!(parent.await, Err(error) if error.is_cancelled()));
    drop(scope);
    assert!(!retired.is_complete());
    let (unblock, worker) = held_files();
    drop(release);
    created.await.unwrap();
    let retained = !original.is_complete();
    let _ = unblock.send(());
    worker.join().unwrap();
    idle_completed(&original).await;
    assert!(retired.is_complete());
    assert!(
        retained,
        "T1034 late lease creation escaped canceled startup"
    );
}

#[tokio::test]
async fn t1034_revoked_startup_rejects_before_native_file_creation() {
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let stop = Arc::new(worker::Stop::default());
    stop.revoke();
    let opened = Arc::new(AtomicBool::new(false));
    let observed = opened.clone();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let startup = spawn(
        Mode::Adaptive,
        stop,
        move || {
            observed.store(true, Ordering::Release);
            anyhow::bail!("T1034 forbidden native creation");
        },
        evidence,
        tracker,
        subjects::Subjects::new(Mode::Adaptive, Arc::new(AtomicU64::new(0))),
        &scope,
    )
    .unwrap();
    let original = startup.task.as_ref().unwrap().completion();
    assert!(startup.finish().await.is_err());
    drop(scope);
    idle_completed(&original).await;
    assert!(!opened.load(Ordering::Acquire));
    assert!(retired.is_complete());
}

#[tokio::test]
async fn t1034_native_startup_panic_retains_original_retirement() {
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let startup = spawn(
        Mode::Adaptive,
        Arc::default(),
        || panic!("T1034 native constructor panic"),
        evidence,
        tracker,
        subjects::Subjects::new(Mode::Adaptive, Arc::new(AtomicU64::new(0))),
        &scope,
    )
    .unwrap();
    let original = startup.task.as_ref().unwrap().completion();
    assert!(startup.finish().await.is_err());
    drop(scope);
    idle_completed(&original).await;
    assert!(retired.is_complete());
}

#[tokio::test]
async fn t1036_encoder_generation_waits_for_original_native_idle_file_cleanup() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let _activity = tracker.encoder_activity(evidence.clone());
    let mut encoder = super::super::encoding::EncoderProcess::default();
    let scope = encoder.begin_generation().unwrap();
    let idle_scope = scope.clone();
    let config = super::super::CaptureConfig {
        encoder: "libx264".into(),
        ..Default::default()
    };
    encoder
        .start_with(&config, (64, 64), false, scope, |_| {
            blent_config::commands::OwnedChild::spawn(
                tokio::process::Command::new("/bin/sleep").arg("60"),
            )
        })
        .unwrap();
    let handle = launch_with(
        lease(directory.path()),
        evidence,
        tracker,
        Arc::new(AtomicU64::new(0)),
        &idle_scope,
    )
    .await
    .unwrap();
    let original = handle.completion();
    drop(idle_scope);
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = ReleaseViewer(release);
    let files = std::thread::spawn(move || {
        let _lock = LEASE_FILES.lock().unwrap();
        entered.send(()).unwrap();
        held.recv().unwrap();
    });
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(handle);
    encoder.stop();
    idle_completed(encoder.pending.as_ref().unwrap()).await;
    let denied = encoder.begin_generation().is_err();
    assert!(!original.is_complete());
    drop(release);
    files.join().unwrap();
    idle_completed(&original).await;
    assert!(encoder.ensure_retired().is_ok());
    assert!(
        denied,
        "T1036 encoder replacement ignored original idle file cleanup"
    );
}

#[tokio::test(start_paused = true)]
async fn t1123_media_clock_advance_does_not_spend_native_retirement_budget() {
    let directory = private_root();
    let tracker = LatencyTracker::new();
    let evidence = evidence(&tracker);
    let _activity = tracker.encoder_activity(evidence.clone());
    let handle = launch(lease(directory.path()), evidence, tracker).await;
    let original = handle.completion();
    let (release, files) = hold_files(|held| {
        let _ = held.recv();
    });
    drop(handle);
    let observed = original.clone();
    let (entered, started) = tokio::sync::oneshot::channel();
    let observing = tokio::spawn(async move {
        entered.send(()).unwrap();
        idle_completed(&observed).await;
    });
    started.await.unwrap();
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::task::yield_now().await;
    let still_observing = !observing.is_finished();
    let retained = !original.is_complete();
    let _ = release.send(());
    files.join().unwrap();
    let outcome = observing.await;
    native_until(|| original.is_complete()).await;
    assert!(
        retained,
        "T1123: native cleanup completed without its original file fence"
    );
    assert!(
        still_observing,
        "T1123: virtual media time exhausted native retirement observation"
    );
    outcome.unwrap();
}

#[tokio::test]
async fn t1115_unknown_view_observations_decline_before_worker_admission() {
    if crate::capture::ownership_tests::isolated(
        "capture::idle::tests::t1115_unknown_view_observations_decline_before_worker_admission",
    ) {
        return;
    }
    let config = CaptureConfig {
        encoder: "libx264".into(),
        ..Default::default()
    };
    let fifo = super::super::fifo_path_for(config.instance).unwrap();
    super::super::helper::ensure_fifo(&fifo).unwrap();
    let tracker = LatencyTracker::new();
    let known = evidence(&tracker);
    let _activity = tracker.encoder_activity(known.clone());
    assert!(
        start_mode(
            Mode::Unchanged,
            &config,
            known,
            tracker,
            Arc::new(AtomicU64::new(0)),
            &Default::default()
        )
        .await
        .is_none(),
        "T1115: unsupported viewer observations consumed an idle worker"
    );
}
