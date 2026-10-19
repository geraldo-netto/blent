//! T1148: timeout is an observation, never original native quiescence.
use super::*;
use std::os::fd::{FromRawFd, OwnedFd};

fn pipe() -> (Arc<File>, File) {
    let mut descriptors = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) },
        0
    );
    let reader = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    let capacity = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) };
    assert!((4096..115200).contains(&capacity));
    (Arc::new(File::from(writer)), File::from(reader))
}
async fn written(reader: &File) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut query = libc::pollfd {
                fd: reader.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut query, 1, 0) } == 1 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
fn failure(
    outcome: Result<Result<Result<()>, tokio::task::JoinError>, tokio::time::error::Elapsed>,
) -> Option<String> {
    match outcome {
        Ok(Ok(Err(error))) => Some(format!("{error:#}")),
        _ => None,
    }
}

#[tokio::test]
async fn t1148_blocked_write_times_out_without_releasing_original_native_work() {
    let (output, reader) = pipe();
    let report = Report::new(
        watch::channel(blent_config::camera::CameraState::Starting).0,
        watch::channel(None).0,
    );
    let (frames, input) = watch::channel(None);
    let (mut controller, native) = start_with(
        output,
        &profile(),
        input,
        &report,
        output_device::configure_fixture,
    )
    .unwrap();
    written(&reader).await; // A real native write has filled the original pipe.
    let outcome = tokio::time::timeout(Duration::from_secs(3), &mut controller).await;
    let message = failure(outcome);
    let retained = !native.is_complete() && report.wait_native().now_or_never().is_none();
    drop(reader); // Independent release wakes the actual blocked kernel writer.
    drop(controller);
    drop(frames);
    report.wait_native().await;
    assert!(native.is_complete());
    assert!(
        retained,
        "T1148 timeout released an unconfirmed native write"
    );
    assert!(
        message.is_some_and(|text| text.contains("write")),
        "T1148 blocked write was not detected"
    );
}

#[tokio::test]
async fn t1148_blocked_configuration_times_out_while_original_lease_stays_owned() {
    let mut rig = Rig::new();
    rig.gate();
    rig.start(configured).unwrap();
    entered(&rig.path("entered")).await;
    let outcome =
        tokio::time::timeout(Duration::from_secs(3), rig.controller.as_mut().unwrap()).await;
    let message = failure(outcome);
    let retained = !rig.native.as_ref().unwrap().is_complete()
        && !reusable(&rig.probe)
        && rig.report.wait_native().now_or_never().is_none();
    rig.settle().await; // Actual original return, join and file release precede assertions.
    assert!(
        retained,
        "T1148 timeout lost the original configuration lease"
    );
    assert!(
        message.is_some_and(|text| text.contains("configuration")),
        "T1148 blocked configuration was not detected"
    );
}
