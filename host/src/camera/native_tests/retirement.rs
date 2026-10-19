//! T1126/T1137: actual owned native output creation/cancellation in private devices.
use super::*;
use futures_util::FutureExt;
use std::{fs::File, io::Write, os::fd::AsRawFd};

pub(super) struct Pending;
impl blent_config::camera::connection::Invitation<CameraProfile> for Pending {
    fn invite<'a>(
        &'a self,
        _: &'a str,
        _: &'a CameraProfile,
    ) -> blent_config::camera::connection::InvitationResult<'a> {
        Box::pin(std::future::pending())
    }
}
impl Connection<CameraProfile> for Pending {
    fn monitor(&self) -> blent_config::camera::connection::InvitationResult<'_> {
        Box::pin(std::future::pending())
    }
    fn close(&self) -> blent_config::camera::connection::InvitationResult<'_> {
        Box::pin(async { Ok(()) })
    }
}

#[test]
fn t1126_camera_reserves_all_producer_observations_before_creation() {
    if private("camera::native_tests::retirement::t1126_camera_reserves_all_producer_observations_before_creation") {
        fixture(|root, front, rear| runtime().block_on(producers(root, front, rear, true)));
    }
}
#[test]
fn t1126_cancelled_camera_keeps_actual_output_workers_observed() {
    if private("camera::native_tests::retirement::t1126_cancelled_camera_keeps_actual_output_workers_observed") {
        fixture(|root, front, rear| runtime().block_on(producers(root, front, rear, false)));
    }
}

async fn started(path: &std::path::Path) {
    loop {
        if std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .count()
            == 2
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
async fn producers(root: &std::path::Path, front: PathBuf, rear: PathBuf, exhaust: bool) {
    let marker = root.join("created");
    // This child contains only this fixture; freeze its port root before workers.
    std::env::set_var("BLENT_T1126_ROOT", root);
    let gate = File::create(root.join("gate")).unwrap();
    assert_eq!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_EX) }, 0);
    let report = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
    let mut reserved = Vec::new();
    if exhaust {
        for _ in 0..4 {
            reserved.push(
                report
                    .reserve_native(crate::camera_control::NativeResource::Producer)
                    .unwrap(),
            );
        }
    }
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    let devices = [
        Arc::new(outputs::open_device(&options.front_device, "Blent Front").unwrap()),
        Arc::new(outputs::open_device(&options.rear_device, "Blent Rear").unwrap()),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let executable = root.join("ffmpeg");
    let token = "ab".repeat(32);
    let session = Session {
        listener: &listener,
        invitation: &Pending,
        token: &token,
        ffmpeg: &executable,
        options: &options,
    };
    let (_stop, mut stopped) = watch::channel(false);
    let work = Box::pin(serve_with(
        session,
        &mut stopped,
        &report,
        &devices,
        gated_start,
    ));
    let blocked = exercise(work, &marker, exhaust, &report).await;
    drop(gate); // Kernel release survives every private-root cleanup path.
    drop(reserved);
    tokio::time::timeout(std::time::Duration::from_secs(3), report.wait_native())
        .await
        .unwrap();
    drop(devices);
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        devices_released(&options),
    )
    .await
    .unwrap();
    assert!(
        blocked,
        "T1126 creation/cancellation escaped original native observation"
    );
}
async fn exercise(
    mut work: std::pin::Pin<Box<impl std::future::Future<Output = Result<()>>>>,
    marker: &std::path::Path,
    exhaust: bool,
    report: &Report,
) -> bool {
    if exhaust {
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), &mut work).await;
        drop(work);
        return matches!(result, Ok(Err(_))) && !marker.exists();
    }
    let ready = tokio::select! {
        _ = &mut work => false,
        result = tokio::time::timeout(std::time::Duration::from_secs(5), started(marker)) => result.is_ok(),
    };
    drop(work); // Abort request is not actual native configuration/write completion.
    ready && report.wait_native().now_or_never().is_none()
}

async fn devices_released(options: &CameraOptions) {
    loop {
        if let (Ok(_front), Ok(_rear)) = (
            outputs::open_device(&options.front_device, "Blent Front"),
            outputs::open_device(&options.rear_device, "Blent Rear"),
        ) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}

#[test]
fn t1126_failed_writer_admission_has_no_unobserved_native_effects() {
    if private("camera::native_tests::retirement::t1126_failed_writer_admission_has_no_unobserved_native_effects") {
        fixture(|root, front, rear| runtime().block_on(partial(root, front, rear)));
    }
}
async fn partial(root: &std::path::Path, front: PathBuf, rear: PathBuf) {
    let mut held = Vec::new();
    for _ in 0..4096 {
        match crate::work_admission::reserve() {
            Ok(admission) => held.push(admission),
            Err(_) => break,
        }
    }
    assert!(crate::work_admission::reserve().is_err());
    std::env::set_var("BLENT_T1126_ROOT", root);
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    let report = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
    let (_frames, input) = watch::channel(None);
    let result = output_worker::start_with(
        Arc::new(outputs::open_device(&options.front_device, "Blent Front").unwrap()),
        &options.profile,
        input,
        &report,
        traced,
    );
    let failed = result.is_err() && !root.join("created").exists();
    drop(result);
    drop(held);
    tokio::time::timeout(std::time::Duration::from_secs(3), report.wait_native())
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        devices_released(&options),
    )
    .await
    .unwrap();
    assert!(
        failed,
        "T1126 exhausted work admission allowed native output effects"
    );
}

fn gated_start(
    lease: Arc<File>,
    profile: &CameraProfile,
    frames: watch::Receiver<Option<Arc<Vec<u8>>>>,
    report: &Report,
) -> Result<(
    crate::owned_task::Task,
    blent_config::retirement::Completion,
)> {
    let cutoff = File::create(controller_gate(&lease)?)?;
    anyhow::ensure!(
        unsafe { libc::flock(cutoff.as_raw_fd(), libc::LOCK_EX) } == 0,
        "T1126 controller gate failure"
    );
    let (mut controller, native) =
        output_worker::start_with(lease, profile, frames, report, gated)?;
    // Task's lease cleanup runs only after actual controller-future destruction
    // and join. The cancellation sender has then closed before native release.
    controller.keep_alive(cutoff);
    Ok((controller, native))
}
fn controller_gate(file: &File) -> Result<PathBuf> {
    let root = PathBuf::from(std::env::var_os("BLENT_T1126_ROOT").unwrap());
    Ok(root.join(format!("controller-{}", file.metadata()?.rdev())))
}

fn traced(file: File, spec: output_device::Spec) -> Result<output_device::ConfiguredOutput> {
    let root = PathBuf::from(std::env::var_os("BLENT_T1126_ROOT").unwrap());
    File::options()
        .append(true)
        .create(true)
        .open(root.join("created"))?
        .write_all(b"created\n")?;
    output_device::configure_fixture(file, spec)
}
fn gated(file: File, spec: output_device::Spec) -> Result<output_device::ConfiguredOutput> {
    let root = PathBuf::from(std::env::var_os("BLENT_T1126_ROOT").unwrap());
    let supervisor = File::open(root.join("gate"))?;
    let cutoff = File::open(controller_gate(&file)?)?;
    let configured = traced(file, spec)?;
    anyhow::ensure!(
        unsafe { libc::flock(cutoff.as_raw_fd(), libc::LOCK_SH) } == 0,
        "T1126 controller fence failure"
    );
    anyhow::ensure!(
        unsafe { libc::flock(supervisor.as_raw_fd(), libc::LOCK_SH) } == 0,
        "T1126 supervisor fence failure"
    );
    Ok(configured)
}

mod cleanup;

mod budget;
