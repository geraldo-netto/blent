//! T1145: original file/work admission and actual blocking-work retirement.
use super::*;
use futures_util::FutureExt;
use std::{
    io,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
};

struct Rig {
    root: tempfile::TempDir,
    probe: File,
    report: Report,
    frames: outputs::Frames,
    input: Option<Frames>,
    controller: Option<Task>,
    native: Option<Completion>,
    gate: Option<File>,
}
impl Rig {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("original");
        File::create(&path).unwrap();
        let probe = File::open(&path).unwrap();
        let (frames, input) = watch::channel(None);
        Self {
            root,
            probe,
            report: Report::new(
                watch::channel(blent_config::camera::CameraState::Starting).0,
                watch::channel(None).0,
            ),
            frames,
            input: Some(input),
            controller: None,
            native: None,
            gate: None,
        }
    }
    fn start(&mut self, configure: Configure) -> Result<()> {
        let original = File::options()
            .read(true)
            .write(true)
            .open(self.path("original"))?;
        assert_eq!(
            unsafe { libc::flock(original.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let (controller, native) = start_with(
            Arc::new(original),
            &profile(),
            self.input.take().unwrap(),
            &self.report,
            configure,
        )?;
        self.controller = Some(controller);
        self.native = Some(native);
        Ok(())
    }
    fn path(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }
    fn gate(&mut self) {
        let gate = File::create(self.path("gate")).unwrap();
        assert_eq!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_EX) }, 0);
        self.gate = Some(gate);
    }
    fn release(&self) {
        if let Some(gate) = &self.gate {
            assert_eq!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_UN) }, 0);
        }
    }
    async fn settle(&mut self) {
        self.release();
        drop(self.controller.take());
        // Protocol budget reports failure; it is not a native-release proof.
        let observed =
            tokio::time::timeout(Duration::from_secs(5), self.report.wait_native()).await;
        if observed.is_err() {
            self.report.wait_native().await;
        }
        assert!(reusable(&self.probe));
        assert!(
            observed.is_ok(),
            "T1145 retirement exceeded fixture protocol budget"
        );
    }
}
impl Drop for Rig {
    fn drop(&mut self) {
        // Every panic/partial setup sponsors release. Original admitted workers
        // retain files/task capacity until actual native return and join.
        self.release();
        drop(self.controller.take());
    }
}
fn profile() -> CameraProfile {
    CameraProfile {
        width: 320,
        height: 240,
        ..Default::default()
    }
}
fn reusable(file: &File) -> bool {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) }, 0);
        return true;
    }
    assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
    false
}
fn configured(output: File, spec: Spec) -> Result<ConfiguredOutput> {
    let path = std::fs::read_link(format!("/proc/self/fd/{}", output.as_raw_fd()))?;
    let root = path.parent().unwrap();
    // Open before publishing entered: the waiter pins its original fence even
    // if its sponsor subsequently removes every pathname in the private root.
    let gate = optional_file(&root.join("gate"))?;
    let pause = optional_file(&root.join("pause"))?;
    File::create(root.join("entered"))?;
    shared_fence(pause)?;
    shared_fence(gate)?;
    if root.join("fail").exists() {
        anyhow::bail!("T1145 native configuration rejected");
    }
    output_device::configure_fixture(output, spec)
}
async fn entered(path: &Path) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
async fn length(path: &Path, minimum: u64) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while std::fs::metadata(path).unwrap().len() < minimum {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1145_cancel_keeps_blocked_original_configuration_and_lease_owned() {
    let mut rig = Rig::new();
    rig.gate();
    rig.start(configured).unwrap();
    entered(&rig.path("entered")).await;
    drop(rig.controller.take());
    tokio::task::yield_now().await;
    let pending = rig.report.wait_native().now_or_never().is_none();
    let held = !reusable(&rig.probe);
    let native_pending = !rig.native.as_ref().unwrap().is_complete();
    rig.settle().await;
    assert!(pending && held && native_pending);
    assert_eq!(
        std::fs::metadata(rig.path("original")).unwrap().len(),
        0,
        "T1145 cancelled setup wrote late data"
    );
}

#[tokio::test]
async fn t1145_native_config_failure_retires_every_original_reference() {
    let mut rig = Rig::new();
    File::create(rig.path("fail")).unwrap();
    rig.start(configured).unwrap();
    let result = rig.controller.take().unwrap().await.unwrap();
    rig.settle().await;
    assert!(result.is_err());
    assert!(rig.native.as_ref().unwrap().is_complete());
}

#[tokio::test]
async fn t1145_initial_black_live_frames_and_stop_black_keep_full_frame_boundaries() {
    let mut rig = Rig::new();
    rig.start(configured).unwrap();
    length(&rig.path("original"), 115200).await;
    let live = Arc::new(vec![99; 115200]);
    rig.frames.send_replace(Some(live.clone()));
    length(&rig.path("original"), 230400).await;
    rig.settle().await;
    let actual = std::fs::read(rig.path("original")).unwrap();
    let black = outputs::blank(&profile()).unwrap();
    assert!(actual.len().is_multiple_of(black.len()));
    assert_eq!(&actual[..black.len()], &black);
    assert!(actual
        .chunks_exact(black.len())
        .any(|frame| frame == &**live));
    assert_eq!(&actual[actual.len() - black.len()..], &black);
}

#[tokio::test]
async fn t1145_invalid_received_frame_lengths_never_reach_native_output() {
    for count in [0, 1, 115199, 115201, 230400] {
        let mut rig = Rig::new();
        rig.frames.send_replace(Some(Arc::new(vec![31; count])));
        rig.start(configured).unwrap();
        let result = rig.controller.take().unwrap().await.unwrap();
        rig.settle().await;
        assert!(result.is_err());
        assert_eq!(std::fs::metadata(rig.path("original")).unwrap().len(), 0);
    }
}

#[tokio::test]
async fn t1145_real_non_video_configuration_failure_does_not_leak_work() {
    let rig = Rig::new();
    let (controller, native) = start(
        Arc::new(File::open("/dev/null").unwrap()),
        &profile(),
        rig.frames.subscribe(),
        &rig.report,
    )
    .unwrap();
    assert!(controller.await.unwrap().is_err());
    rig.report.wait_native().await;
    assert!(native.is_complete());
}

fn optional_file(path: &Path) -> io::Result<Option<File>> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
fn shared_fence(file: Option<File>) -> io::Result<()> {
    if let Some(file) = file {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
struct Recovery {
    path: PathBuf,
    pause: File,
}
impl Drop for Recovery {
    fn drop(&mut self) {
        // The deliberate historical-fault branch has its own recovery sponsor.
        let _ = std::fs::create_dir_all(&self.path);
        let _ = File::create(self.path.join("release"));
        let _ = unsafe { libc::flock(self.pause.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[tokio::test]
async fn t1145_fixture_owner_loss_cannot_erase_the_only_native_release() {
    let mut rig = Rig::new();
    rig.gate();
    let pause = File::create(rig.path("pause")).unwrap();
    assert_eq!(unsafe { libc::flock(pause.as_raw_fd(), libc::LOCK_EX) }, 0);
    let recovery = Recovery {
        path: rig.root.path().to_path_buf(),
        pause,
    };
    rig.start(configured).unwrap();
    entered(&rig.path("entered")).await;
    let report = rig.report.clone();
    drop(rig); // Delete the old release marker while the native reader is fenced.
    assert_eq!(
        unsafe { libc::flock(recovery.pause.as_raw_fd(), libc::LOCK_UN) },
        0
    );
    let settled = tokio::time::timeout(Duration::from_secs(2), report.wait_native())
        .await
        .is_ok();
    let path = recovery.path.clone();
    drop(recovery); // Recover before awaiting actual settlement even against bad code.
    report.wait_native().await;
    std::fs::remove_dir_all(path).unwrap();
    assert!(
        settled,
        "T1145 fixture owner erased native release before settlement"
    );
}

#[tokio::test]
async fn t1145_both_original_observations_are_reserved_before_native_configuration() {
    for count in [3, 4] {
        let mut rig = Rig::new();
        let slots: Vec<_> = (0..count)
            .map(|_| rig.report.reserve_native(NativeResource::Producer).unwrap())
            .collect();
        assert!(rig.start(configured).is_err());
        assert!(!rig.path("entered").exists());
        drop(slots);
        rig.settle().await;
    }
}

#[tokio::test]
async fn t1145_work_admission_failure_releases_originals_without_native_effects() {
    let mut held = Vec::new();
    for _ in 0..4096 {
        match crate::work_admission::reserve() {
            Ok(admission) => held.push(admission),
            Err(_) => break,
        }
    }
    assert!(crate::work_admission::reserve().is_err());
    let mut rig = Rig::new();
    let result = rig.start(configured);
    let no_effects = !rig.path("entered").exists();
    drop(held);
    rig.settle().await;
    assert!(result.is_err() && no_effects);
}

#[tokio::test]
async fn t1145_rejected_controller_admission_still_retires_its_original_native_work() {
    let mut held = Vec::new();
    for _ in 0..4096 {
        match crate::work_admission::reserve() {
            Ok(admission) => held.push(admission),
            Err(_) => break,
        }
    }
    // Exactly one work slot: native creation is admitted, its controller is not.
    assert!(crate::work_admission::reserve().is_err());
    drop(held.pop().unwrap());
    let mut rig = Rig::new();
    rig.gate();
    let result = rig.start(configured);
    drop(held);
    rig.settle().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn t1145_cancelled_unstarted_job_never_configures_or_writes() {
    let rig = Rig::new();
    let (running, stopped) = watch::channel(false);
    drop(running);
    let job = Job {
        lease: Arc::new(File::open(rig.path("original")).unwrap()),
        spec: Spec::from_profile(&profile()).unwrap(),
        black: outputs::blank(&profile()).unwrap(),
        frames: rig.frames.subscribe(),
        stopped,
        configure: configured,
        operation: watch::channel(None).0,
    };
    Task::spawn_blocking(move || job.run())
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(!rig.path("entered").exists());
    assert_eq!(std::fs::metadata(rig.path("original")).unwrap().len(), 0);
}

#[tokio::test]
async fn t1145_closed_frame_channel_publishes_final_black_then_joins() {
    let mut rig = Rig::new();
    rig.start(configured).unwrap();
    length(&rig.path("original"), 115200).await;
    let frames = std::mem::replace(&mut rig.frames, watch::channel(None).0);
    drop(frames);
    let result = rig.controller.take().unwrap().await.unwrap();
    rig.settle().await;
    assert!(result.is_ok());
    let bytes = std::fs::read(rig.path("original")).unwrap();
    assert_eq!(
        &bytes[bytes.len() - 115200..],
        outputs::blank(&profile()).unwrap()
    );
}

#[tokio::test]
async fn t1145_real_native_write_failure_retires_without_followup_writes() {
    let rig = Rig::new();
    let file = File::options().write(true).open("/dev/full").unwrap();
    let (controller, native) = start_with(
        Arc::new(file),
        &profile(),
        rig.frames.subscribe(),
        &rig.report,
        output_device::configure_fixture,
    )
    .unwrap();
    let result = controller.await.unwrap();
    rig.report.wait_native().await;
    assert!(result.is_err());
    assert!(native.is_complete());
}

struct Unlock(File);
impl Drop for Unlock {
    fn drop(&mut self) {
        assert_eq!(unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) }, 0);
    }
}

#[test]
fn t1145_runtime_teardown_retains_blocked_native_ownership_until_exact_return() {
    let (handoff, received) = std::sync::mpsc::sync_channel(1);
    let owner = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        // Rig unwinds before runtime on partial setup failure.
        let mut rig = Rig::new();
        rig.gate();
        runtime.block_on(async {
            rig.start(configured).unwrap();
            entered(&rig.path("entered")).await;
        });
        let release = Unlock(rig.gate.as_ref().unwrap().try_clone().unwrap());
        handoff
            .send((rig.report.clone(), rig.probe.try_clone().unwrap(), release))
            .unwrap();
        drop(runtime); // Native configuration still owns the pinned original fence.
    });
    let (report, probe, release) = received.recv().unwrap();
    let held = !reusable(&probe);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let pending = runtime.block_on(async { report.wait_native().now_or_never().is_none() });
    drop(release);
    owner.join().unwrap();
    runtime.block_on(report.wait_native());
    assert!(held && pending);
    assert!(reusable(&probe));
}

#[tokio::test]
async fn t1145_invalid_profile_boundaries_never_admit_native_configuration() {
    for width in [0, 1, 159, 161, 1921, u32::MAX] {
        let rig = Rig::new();
        let shape = CameraProfile { width, ..profile() };
        let result = start_with(
            Arc::new(File::open(rig.path("original")).unwrap()),
            &shape,
            rig.frames.subscribe(),
            &rig.report,
            configured,
        );
        assert!(result.is_err());
        assert!(!rig.path("entered").exists());
        rig.report.wait_native().await;
    }
}

mod operation_tests;
