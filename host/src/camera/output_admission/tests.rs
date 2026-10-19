//! T1147: bounded invalid paths, admission exhaustion and queued-result cleanup.
use super::*;
use futures_util::FutureExt;
use std::os::fd::AsRawFd;

fn report() -> Report {
    Report::new(
        watch::channel(blent_config::camera::CameraState::Starting).0,
        watch::channel(None).0,
    )
}
#[tokio::test]
async fn t1147_pre_stopped_and_closed_control_admit_no_native_work() {
    for closed in [false, true] {
        let (sender, mut stop) = watch::channel(!closed);
        if closed {
            drop(sender);
        }
        let report = report();
        assert!(admit(&CameraOptions::default(), &mut stop, &report)
            .await
            .unwrap()
            .is_none());
        assert!(report.wait_native().now_or_never().is_some());
    }
}
#[test]
fn t1147_closed_reply_rejects_before_any_configured_path_access() {
    let (_sender, stop) = watch::channel(false);
    let (ready, reply) = oneshot::channel();
    drop(reply);
    let paths = [
        PathBuf::from("/T1147/missing"),
        PathBuf::from("/T1147/unused"),
    ];
    assert!(open_pair(&paths, &stop, &ready).unwrap().is_none());
}
#[tokio::test]
async fn t1147_invalid_path_corpus_retires_constructor_and_close_credit() {
    let root = tempfile::tempdir().unwrap();
    let regular = root.path().join("regular");
    std::fs::write(&regular, "").unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&regular, &link).unwrap();
    let mut paths = vec![
        PathBuf::new(),
        root.path().join("missing"),
        root.path().to_owned(),
        regular,
        link,
        PathBuf::from("/dev/null"),
        PathBuf::from("x".repeat(4097)),
    ];
    use std::os::unix::ffi::OsStringExt;
    for byte in 0..=255 {
        paths.push(root.path().join(std::ffi::OsString::from_vec(vec![byte])));
    }
    for count in [0, 1, 255, 256, 4095, 4096, 4097] {
        paths.push(root.path().join("X".repeat(count)));
    }
    for path in paths {
        let mut options = CameraOptions::default();
        options.front_device = path;
        let (_sender, mut stop) = watch::channel(false);
        let report = report();
        let result = admit(&options, &mut stop, &report).await;
        assert!(result.is_err());
        drop(result);
        report.wait_native().await;
    }
}
#[tokio::test]
async fn t1147_observation_and_work_exhaustion_leave_no_unbound_obligation() {
    let report = report();
    let first = report
        .reserve_native(control::NativeResource::Devices)
        .unwrap();
    let second = report
        .reserve_native(control::NativeResource::Devices)
        .unwrap();
    let (_sender, mut stop) = watch::channel(false);
    assert!(admit(&CameraOptions::default(), &mut stop, &report)
        .await
        .is_err());
    drop((first, second));
    report.wait_native().await;
    let mut held = Vec::new();
    while let Ok(credit) = crate::work_admission::reserve() {
        held.push(credit);
        assert!(held.len() <= 4096);
    }
    for _ in 0..2 {
        assert!(admit(&CameraOptions::default(), &mut stop, &report)
            .await
            .is_err());
        report.wait_native().await;
        drop(held.pop()); // Zero, then one credit: fail lease, then creator admission.
    }
    drop(held);
}
#[tokio::test]
async fn t1147_abandoned_queued_devices_keep_funded_original_file_retirement() {
    let root = tempfile::tempdir().unwrap();
    let paths = [root.path().join("front"), root.path().join("rear")];
    let files = paths.clone().map(|path| {
        let file = File::create(path).unwrap();
        assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) }, 0);
        Arc::new(file)
    });
    let report = report();
    let original = report
        .reserve_native(control::NativeResource::Devices)
        .unwrap();
    let admission = crate::work_admission::reserve().unwrap();
    let completion = admission.completion();
    original.bind(completion.clone());
    let devices = Devices(Some(Lease { files, admission }));
    let (ready, reply) = oneshot::channel();
    assert!(ready.send(devices).is_ok());
    assert!(!completion.is_complete());
    drop(reply); // Queued ownership loss still has its pre-admitted close sponsor.
    report.wait_native().await;
    assert!(completion.is_complete());
    for path in paths {
        let probe = File::open(path).unwrap();
        assert_eq!(
            unsafe { libc::flock(probe.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
    }
}
