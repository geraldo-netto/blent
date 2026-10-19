//! T1137: a successful stop cannot hide a failed original final-frame write.
use super::*;
use std::os::fd::{FromRawFd, OwnedFd};

fn endpoint() -> (Arc<File>, tokio::net::UnixDatagram) {
    let mut descriptors = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                descriptors.as_mut_ptr(),
            )
        },
        0
    );
    let output = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let input = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
    let receiver = std::os::unix::net::UnixDatagram::from(input);
    receiver.set_nonblocking(true).unwrap();
    (
        Arc::new(File::from(output)),
        tokio::net::UnixDatagram::from_std(receiver).unwrap(),
    )
}
fn start(
    lease: Arc<File>,
    profile: &CameraProfile,
    frames: watch::Receiver<Option<Arc<Vec<u8>>>>,
    report: &Report,
) -> Result<(
    crate::owned_task::Task,
    blent_config::retirement::Completion,
)> {
    output_worker::start_with(
        lease,
        profile,
        frames,
        report,
        output_device::configure_fixture,
    )
}

#[tokio::test]
async fn t1137_successful_stop_cannot_hide_original_final_black_write_failure() {
    let (front, front_reader) = endpoint();
    let (rear, rear_reader) = endpoint();
    let devices = [front, rear];
    let readers = [front_reader, rear_reader];
    let options = tests::options();
    let bytes = options.frame_bytes().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (stop, mut stopped) = watch::channel(false);
    let report = tests::new_report();
    let session = Session {
        listener: &listener,
        invitation: &Pending,
        token: "unused-no-tablet",
        ffmpeg: std::path::Path::new("/unused-decoder"),
        options: &options,
    };
    let fail_cleanup = async move {
        let mut frame = vec![0; bytes + 1];
        for reader in &readers {
            assert_eq!(reader.recv(&mut frame).await.unwrap(), bytes);
        }
        drop(readers); // Native startup succeeded; only the final write must fail.
        stop.send_replace(true);
    };
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            serve_with(session, &mut stopped, &report, &devices, start),
            fail_cleanup
        )
    })
    .await;
    report.wait_native().await;
    drop(devices);
    let (result, ()) = outcome.unwrap();
    assert!(
        result.is_err(),
        "T1137 successful stop hid final native black-frame failure"
    );
}
