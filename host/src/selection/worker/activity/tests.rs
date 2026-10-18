//! T1037: original activity fences cancel work; actual cleanup fences restart.
use super::super::*;
use super::*;
use blent_config::host_activity::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::timeout;

fn state(feed: &Feed) -> Publication {
    Publication::Observed {
        capabilities: Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        snapshot: feed.snapshot(true, false),
    }
}
fn feed() -> Feed {
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    observe(&mut feed, 1, false);
    feed
}
fn observe(feed: &mut Feed, revision: u64, locked: bool) {
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
#[tokio::test]
async fn t1037_coalesced_pause_resume_cancels_the_original_pending_trial() {
    let (settings, mut updates) = watch::channel(super::super::tests::settings());
    let key = settings.borrow().selection_key();
    let (_visible, mut display) = watch::channel(true);
    let (_owner, mut stop) = watch::channel(false);
    let mut feed = feed();
    let (publish, source) = watch::channel(state(&feed));
    let activity = blent::display_work::View::new(Some(source), None, true);
    let attachment = crate::attachment::Attachment::new(settings);
    let authority = TrialAuthority {
        activity: &activity,
        work: activity.begin().unwrap(),
        profile: Box::new(attachment.lease()),
    };
    let work = async {
        observe(&mut feed, 2, true);
        publish.send_replace(state(&feed));
        observe(&mut feed, 3, false);
        publish.send_replace(state(&feed));
        std::future::pending::<()>().await;
    };
    let result = timeout(
        Duration::from_millis(100),
        until_changed(
            &key,
            &mut updates,
            &mut display,
            &mut stop,
            &authority,
            work,
        ),
    )
    .await;
    assert!(
        matches!(result, Ok(None)),
        "T1037 coalesced stop fence did not cancel selection"
    );
}
#[tokio::test]
async fn t1037_ready_old_result_cannot_complete_after_activity_stops() {
    let (settings, mut updates) = watch::channel(super::super::tests::settings());
    let key = settings.borrow().selection_key();
    let (_visible, mut display) = watch::channel(true);
    let (_owner, mut stop) = watch::channel(false);
    let (publish, source) = watch::channel(state(&feed()));
    let activity = blent::display_work::View::new(Some(source), None, true);
    let attachment = crate::attachment::Attachment::new(settings);
    let authority = TrialAuthority {
        activity: &activity,
        work: activity.begin().unwrap(),
        profile: Box::new(attachment.lease()),
    };
    let work = async {
        publish.send_replace(Publication::Failed);
        7
    };
    assert_eq!(
        until_changed(
            &key,
            &mut updates,
            &mut display,
            &mut stop,
            &authority,
            work
        )
        .await,
        None
    );
}
#[tokio::test]
async fn t1037_invalidated_activity_cannot_poll_even_first_work_effect() {
    for invalid in [
        Publication::Pending,
        Publication::Failed,
        Publication::Stopped,
    ] {
        let (settings, mut updates) = watch::channel(super::super::tests::settings());
        let key = settings.borrow().selection_key();
        let (_visible, mut display) = watch::channel(true);
        let (_owner, mut stop) = watch::channel(false);
        let (publish, source) = watch::channel(state(&feed()));
        let activity = blent::display_work::View::new(Some(source), None, true);
        let attachment = crate::attachment::Attachment::new(settings);
        let authority = TrialAuthority {
            activity: &activity,
            work: activity.begin().unwrap(),
            profile: Box::new(attachment.lease()),
        };
        publish.send_replace(invalid);
        let effects = AtomicUsize::new(0);
        let work = async {
            effects.fetch_add(1, Ordering::SeqCst);
        };
        let result = until_changed(
            &key,
            &mut updates,
            &mut display,
            &mut stop,
            &authority,
            work,
        )
        .await;
        assert!(result.is_none());
        assert_eq!(effects.load(Ordering::SeqCst), 0);
    }
}
struct Release(std::sync::mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[tokio::test]
async fn t1037_retirement_waits_for_original_environment_creator_after_parent_cancellation() {
    let (scope, original) = blent::native_scope::Scope::default().group();
    let creator = scope.clone();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = Release(release);
    let task = tokio::spawn(async move {
        creator
            .blocking(move |scope| {
                drop(scope);
                let _ = entered.send(());
                let _ = held.recv();
            })
            .await
    });
    ready.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(scope);
    let (_owner, mut stop) = watch::channel(false);
    let mut wait = Box::pin(retired(&original, &mut stop));
    let early = timeout(Duration::from_millis(20), &mut wait).await;
    drop(release);
    let retained = early.is_err();
    let actual = match early {
        Ok(result) => result,
        Err(_) => timeout(Duration::from_secs(5), wait).await.unwrap(),
    };
    assert!(
        retained,
        "T1037 parent cancellation permitted a replacement before actual creator join"
    );
    assert!(actual);
    assert!(original.is_complete());
}

#[tokio::test]
async fn t1037_profile_change_is_armed_before_read_and_wakes_without_metadata_change() {
    use blent_config::connection::routes::{Identity, Link};
    let (settings, mut metadata) = watch::channel(super::super::tests::settings());
    let attachment = crate::attachment::Attachment::new(settings.clone());
    let identity = Identity::observed("T1037-tablet".into(), Some("T1037-tablet".into())).unwrap();
    attachment.begin_route(Some(identity.clone()), Some(Link::Usb));
    settings.send_replace(super::super::tests::settings());
    metadata.borrow_and_update();
    let source: &dyn ProfileSource = &attachment;
    let notice = source.changed();
    let original = source.profile();
    attachment.begin_route(Some(identity), Some(Link::Network));
    assert!(!original.active());
    assert!(source.profile().active());
    assert!(
        !metadata.has_changed().unwrap(),
        "T1037 fixture must isolate profile wake from settings updates"
    );
    timeout(Duration::from_millis(20), notice)
        .await
        .expect("T1037 original profile change was lost");
}

async fn selected(updates: &mut watch::Receiver<EncoderSettings>) {
    timeout(Duration::from_secs(2), async {
        loop {
            if updates.borrow_and_update().selection.is_some() {
                return;
            }
            updates.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t1037_controller_observes_pending_resume_coalesced_fences_and_completed_work() {
    if isolated_controller("selection::worker::activity::tests::t1037_controller_observes_pending_resume_coalesced_fences_and_completed_work") {
        return;
    }
    let snapshot = super::super::tests::settings();
    let (settings, mut updates) = watch::channel(snapshot);
    let attachment = crate::attachment::Attachment::new(settings.clone());
    let (_visible, display) = watch::channel(true);
    let (owner, stop) = watch::channel(false);
    let (publish, source) = watch::channel(Publication::Pending);
    let task = tokio::spawn(run(
        CaptureConfig {
            instance: u32::MAX,
            gpu_policy: blent_config::gpu::Policy::Software,
            ..Default::default()
        },
        settings.clone(),
        display,
        stop,
        Default::default(),
        Arc::new(attachment),
        Default::default(),
        Arc::new(blent::display_work::View::new(Some(source), None, true)),
        None,
    ));
    assert!(timeout(Duration::from_millis(20), updates.changed())
        .await
        .is_err());
    assert!(settings.borrow().selection.is_none());
    let mut feed = feed();
    publish.send_replace(state(&feed));
    selected(&mut updates).await;
    let first = settings.borrow().selection_key();
    // A harmless newer observation must not repeat a completed trial.
    observe(&mut feed, 2, false);
    publish.send_replace(state(&feed));
    assert!(timeout(Duration::from_millis(20), updates.changed())
        .await
        .is_err());
    observe(&mut feed, 3, true);
    publish.send_replace(state(&feed));
    observe(&mut feed, 4, false);
    publish.send_replace(state(&feed));
    timeout(Duration::from_secs(2), updates.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.borrow().selection_key(), first);
    owner.send_replace(true);
    timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[derive(Default)]
struct Report(std::sync::Mutex<Vec<blent_config::retirement::Completion>>);
impl blent::capture_contract::NativeRetirement for Report {
    fn retain(&self, proof: blent_config::retirement::Completion) {
        self.0.lock().unwrap().push(proof);
    }
}
async fn first_proof(report: &Report) -> blent_config::retirement::Completion {
    timeout(Duration::from_secs(5), async {
        loop {
            if let Some(proof) = report.0.lock().unwrap().first().cloned() {
                return proof;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap()
}
async fn writer(path: &std::path::Path) -> std::fs::File {
    use std::os::unix::fs::OpenOptionsExt;
    timeout(Duration::from_secs(30), async {
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)
            {
                Ok(writer) => return writer,
                Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {
                    tokio::time::sleep(Duration::from_millis(1)).await
                }
                Err(error) => panic!("T1037 FIFO writer: {error}"),
            }
        }
    })
    .await
    .unwrap()
}
async fn joined(report: &Report) {
    timeout(Duration::from_secs(5), async {
        loop {
            if report
                .0
                .lock()
                .unwrap()
                .iter()
                .all(|proof| proof.is_complete())
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t1037_controller_cannot_overlap_original_cache_preparation_or_lose_late_native_proof() {
    let mut preparation = Preparation::new();
    let (_visible, display) = watch::channel(true);
    let (owner, stop) = watch::channel(false);
    let mut feed = feed();
    let (publish, source) = watch::channel(state(&feed));
    let report = Arc::new(Report::default());
    let scope = blent::native_scope::Scope::new(Some(report.clone()));
    let task = tokio::spawn(run(
        preparation.config.clone(),
        preparation.settings.clone(),
        display,
        stop,
        Default::default(),
        Arc::new(preparation.attachment.clone()),
        scope,
        Arc::new(blent::display_work::View::new(Some(source), None, true)),
        None,
    ));
    let original = first_proof(&report).await;
    preparation.hold().await; // Actual preflight reader now blocks on the original FIFO.
    observe(&mut feed, 2, true);
    publish.send_replace(state(&feed));
    observe(&mut feed, 3, false);
    publish.send_replace(state(&feed));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let overlapping = report.0.lock().unwrap().len() != 1;
    let held = !original.is_complete();
    owner.send_replace(true);
    timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    let retained_after_parent = !original.is_complete();
    preparation.release();
    joined(&report).await;
    assert!(
        !overlapping,
        "T1037 resumed selection overlapped its original cache preparation"
    );
    assert!(
        held && retained_after_parent,
        "T1037 returned controller lost original native preparation"
    );
    assert!(original.is_complete());
    assert!(
        report.0.lock().unwrap().len() >= 2,
        "T1037 actual late version-query child was not retained"
    );
}

// Isolate the harmless failing encoder adapter without changing global process state.
fn isolated_controller(test: &str) -> bool {
    if std::env::var_os("BLENT_T1037_CONTROLLER_CHILD").is_some() {
        return false;
    }
    use blent_config::commands::SyncCommandExt;
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let ffmpeg = root.path().join("ffmpeg");
    std::fs::write(&ffmpeg, b"#!/bin/sh\nexit 17\n").unwrap();
    std::fs::set_permissions(ffmpeg, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("BLENT_T1037_CONTROLLER_CHILD", "1")
        .env("PATH", root.path())
        .output_timeout(Duration::from_secs(20))
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

// Release both an already-blocked reader and a creator which opens only after
// cancellation. Rename first while a nonblocking rescue writer admits old opens.
struct Fingerprint {
    path: std::path::PathBuf,
    fallback: std::path::PathBuf,
    writer: Option<std::fs::File>,
}
impl Drop for Fingerprint {
    fn drop(&mut self) {
        use std::os::unix::fs::OpenOptionsExt;
        let rescue = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&self.path);
        let _ = std::fs::rename(&self.fallback, &self.path);
        drop(self.writer.take());
        drop(rescue);
    }
}

#[tokio::test]
async fn t1037_profile_handoff_cannot_accept_old_result_with_unchanged_selection_key() {
    use blent_config::connection::routes::{Identity, Link};
    let (settings, mut updates) = watch::channel(super::super::tests::settings());
    let attachment = crate::attachment::Attachment::new(settings.clone());
    let identity = Identity::observed("T1037-result".into(), Some("T1037-result".into())).unwrap();
    attachment.begin_route(Some(identity.clone()), Some(Link::Usb));
    settings.send_replace(super::super::tests::settings());
    let key = settings.borrow().selection_key();
    let (_visible, mut display) = watch::channel(true);
    let (_owner, mut stop) = watch::channel(false);
    let authority = TrialAuthority::legacy(&attachment);
    let work = async {
        attachment.begin_route(Some(identity), Some(Link::Network));
        9
    };
    let result = until_changed(
        &key,
        &mut updates,
        &mut display,
        &mut stop,
        &authority,
        work,
    )
    .await;
    assert_eq!(settings.borrow().selection_key(), key);
    assert_eq!(
        result, None,
        "T1037 old attachment completed under unchanged metadata"
    );
}

#[tokio::test]
async fn t1045_selector_uses_original_viewer_demand_and_saved_preference() {
    if isolated_controller("selection::worker::activity::tests::t1045_selector_uses_original_viewer_demand_and_saved_preference") {
        return;
    }
    for pause in [true, false] {
        let (video, initial) = blent::video_queue::channel(8, Default::default());
        drop(initial);
        let (settings, mut updates) = watch::channel(super::super::tests::settings());
        let attachment = crate::attachment::Attachment::new(settings.clone());
        let (_visible, display) = watch::channel(true);
        let (owner, stop) = watch::channel(false);
        let work = Arc::new(blent::display_work::View::new(
            None,
            Some(video.viewer_activity()),
            pause,
        ));
        let task = tokio::spawn(run(
            CaptureConfig {
                instance: u32::MAX,
                gpu_policy: blent_config::gpu::Policy::Software,
                ..Default::default()
            },
            settings.clone(),
            display,
            stop,
            Default::default(),
            Arc::new(attachment),
            Default::default(),
            work,
            None,
        ));
        if pause {
            assert!(
                timeout(Duration::from_millis(20), updates.changed())
                    .await
                    .is_err(),
                "T1045 empty session calibrated"
            );
        } else {
            selected(&mut updates).await;
        }
        let receiver = video.subscribe();
        selected(&mut updates).await;
        updates.borrow_and_update();
        let second = video.subscribe();
        drop(second);
        assert!(
            timeout(Duration::from_millis(20), updates.changed())
                .await
                .is_err(),
            "T1045 continuous viewers repeated calibration"
        );
        drop(receiver);
        let _replacement = video.subscribe();
        if pause {
            timeout(Duration::from_secs(2), updates.changed())
                .await
                .unwrap()
                .unwrap();
        } else {
            assert!(timeout(Duration::from_millis(20), updates.changed())
                .await
                .is_err());
        }
        owner.send_replace(true);
        timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
    }
}

// Own the FIFO, rescue writer and temporary directory together. Native readers
// retire before directory reuse; release remains available on panic through Drop.
struct Preparation {
    fingerprint: Option<Fingerprint>,
    config: CaptureConfig,
    settings: watch::Sender<crate::media::EncoderSettings>,
    attachment: crate::attachment::Attachment,
    _directory: tempfile::TempDir,
}
impl Preparation {
    fn new() -> Self {
        use blent_config::connection::routes::{Identity, Link};
        use std::os::unix::ffi::OsStrExt;
        let directory = tempfile::tempdir().unwrap();
        let fifo = directory.path().join("held-helper-fingerprint");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let fallback = directory.path().join("retired-helper-fingerprint");
        std::fs::write(&fallback, b"T1037 harmless retired fingerprint").unwrap();
        let fingerprint = Fingerprint {
            path: fifo.clone(),
            fallback,
            writer: None,
        };
        let (mut snapshot, _) = super::super::cache::tests::setup();
        snapshot.decoder_epoch = 7;
        let (settings, _) = watch::channel(snapshot.clone());
        let attachment = crate::attachment::Attachment::new(settings.clone());
        attachment.begin_route(
            Some(
                Identity::observed("T1037-preparation".into(), Some("T1037-preparation".into()))
                    .unwrap(),
            ),
            Some(Link::Usb),
        );
        settings.send_replace(snapshot);
        Self {
            fingerprint: Some(fingerprint),
            config: CaptureConfig {
                instance: u32::MAX,
                helper_path: fifo,
                gpu_policy: blent_config::gpu::Policy::Software,
                ..Default::default()
            },
            settings,
            attachment,
            _directory: directory,
        }
    }
    async fn hold(&mut self) {
        self.fingerprint.as_mut().unwrap().writer = Some(writer(&self.config.helper_path).await);
    }
    fn release(&mut self) {
        drop(self.fingerprint.take());
    }
}

#[tokio::test]
async fn t1050_selector_receipt_waits_for_original_late_native_preparation() {
    let mut preparation = Preparation::new();
    let (control, mut participants) = blent::display_quiescence::channel(1).unwrap();
    let participant = participants.pop().unwrap();
    let work =
        Arc::new(blent::display_work::View::new(None, None, false).with_gate(participant.source()));
    let (_visible, display) = watch::channel(true);
    let (owner, stop) = watch::channel(false);
    let report = Arc::new(Report::default());
    let scope = blent::native_scope::Scope::new(Some(report.clone()));
    let task = tokio::spawn(run(
        preparation.config.clone(),
        preparation.settings.clone(),
        display,
        stop,
        Default::default(),
        Arc::new(preparation.attachment.clone()),
        scope,
        work,
        Some(participant),
    ));
    let original = first_proof(&report).await;
    preparation.hold().await;
    let fence = control.pause();
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !fence.complete(),
        "T1050 selector reported before native preparation retired"
    );
    assert!(!control.resume(&fence));
    assert!(!original.is_complete());
    preparation.release();
    timeout(Duration::from_secs(10), async {
        while !fence.complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(original.is_complete());
    assert!(report
        .0
        .lock()
        .unwrap()
        .iter()
        .all(|proof| proof.is_complete()));
    assert!(control.resume(&fence));
    owner.send_replace(true);
    task.await.unwrap();
    joined(&report).await;
}
