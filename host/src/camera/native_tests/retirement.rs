//! T1126: actual owned feeder/child creation and cancellation in private devices.
use super::*;
use futures_util::FutureExt;

struct Pending;
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
fn t1126_cancelled_camera_keeps_actual_feeders_and_children_observed() {
    if private("camera::native_tests::retirement::t1126_cancelled_camera_keeps_actual_feeders_and_children_observed") {
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
    tests::script(
        root,
        "ffmpeg",
        &format!(
            "import sys\nwith open({marker:?}, 'a') as output: output.write('created\\n')\n{}",
            if exhaust {
                "sys.exit(0)"
            } else {
                "sys.stdin.buffer.read()"
            }
        ),
    );
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
    let work = Box::pin(serve(session, &mut stopped, &report, &devices));
    let blocked = exercise(work, &marker, exhaust, &report).await;
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
    drop(work); // Abort request is not actual feeder/child completion.
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
fn t1126_failed_feeder_admission_retires_the_already_created_original_child() {
    if private("camera::native_tests::retirement::t1126_failed_feeder_admission_retires_the_already_created_original_child") {
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
    tests::script(root, "ffmpeg", "import sys\nsys.stdin.buffer.read()");
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    let report = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
    let (_frames, input) = watch::channel(None);
    let result = producer_work(
        &root.join("ffmpeg"),
        &options.front_device,
        &options,
        input,
        Arc::new(outputs::open_device(&options.front_device, "Blent Front").unwrap()),
        &report,
    );
    let failed = result.is_err();
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
        "T1126 exhausted work admission started a feeder task"
    );
}
