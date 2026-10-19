//! T1147: real delayed sysfs admission must not stop the async coordinator.
use super::*;
use futures_util::FutureExt;
use std::{
    fs::File,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    sync::mpsc,
    time::Duration,
};

const TEST: &str = "camera::native_tests::startup::t1147_stop_during_original_open_keeps_coordinator_responsive_and_skips_later_admission";

#[test]
fn t1147_stop_during_original_open_keeps_coordinator_responsive_and_skips_later_admission() {
    if private(TEST) {
        fixture(delayed);
    }
}
fn name(root: &std::path::Path, path: &std::path::Path) -> PathBuf {
    let device = std::fs::metadata(path).unwrap().rdev();
    root.join(format!(
        "sys/{}:{}/name",
        libc::major(device),
        libc::minor(device)
    ))
}
fn delayed(root: &std::path::Path, front: PathBuf, rear: PathBuf) {
    let front_name = name(root, &front);
    std::fs::remove_file(&front_name).unwrap();
    let cpath = std::ffi::CString::new(front_name.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    let mut rear_watch = opened(&name(root, &rear));
    tools(root);
    std::env::set_var("PATH", root); // Only this single-test private child.
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    let (stop, stopped) = watch::channel(false);
    let (acknowledge, acknowledged) = mpsc::channel();
    let (done, settled) = mpsc::channel();
    let writer_stop = stop.clone();
    let writer_path = front_name.clone();
    let writer =
        std::thread::spawn(move || release(writer_path, writer_stop, acknowledged, settled));
    let report = tests::new_report();
    let result = runtime().block_on(async {
        let attempt = async {
            let result = run_attempt(options.clone(), stopped.clone(), report.clone()).await;
            let _ = done.send(());
            stop.send_replace(true); // Also settles the observer after early rejection.
            result
        };
        let observe = async {
            let mut observed = stopped.clone();
            control::cancelled(&mut observed).await;
            let pending = report.wait_native().now_or_never().is_none();
            let _ = acknowledge.send(pending);
        };
        let (result, ()) = tokio::join!(attempt, observe);
        report.wait_native().await;
        result
    });
    let released = writer.join().unwrap();
    let later_endpoint = event(&mut rear_watch);
    std::fs::remove_file(&front_name).unwrap();
    std::fs::write(front_name, "Blent Front\n").unwrap();
    drop(outputs::open_device(&options.front_device, "Blent Front").unwrap());
    drop(outputs::open_device(&options.rear_device, "Blent Rear").unwrap());
    assert!(
        released.0,
        "T1147 Stop could not run while original sysfs read was blocked"
    );
    assert!(
        released.1,
        "T1147 original native admission was not retained through cancellation"
    );
    released.2.unwrap();
    result.unwrap();
    assert!(
        !later_endpoint,
        "T1147 admitted a later endpoint after Stop"
    );
    assert!(
        !root.join("called-adb").exists(),
        "T1147 created a connection after Stop"
    );
}
fn tools(root: &std::path::Path) {
    tests::script(root, "ffmpeg", "import sys\nsys.exit(1)");
    tests::script(
        root,
        "adb",
        &format!(
            r#"import pathlib, sys
root = pathlib.Path({root:?})
(root/'called-adb').touch()
a = sys.argv
if 'tcp:0' in a:
    (root/'mapping').write_text(a[-1]); print('34567')
elif '--list' in a: print('Usb tcp:34567 '+(root/'mapping').read_text())
elif '--remove' in a: (root/'mapping').unlink()
else: print('fixture-tablet')
"#
        ),
    );
}
fn opened(path: &std::path::Path) -> File {
    let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
    assert!(fd >= 0);
    let file = unsafe { File::from_raw_fd(fd) };
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert!(
        unsafe { libc::inotify_add_watch(file.as_raw_fd(), path.as_ptr(), libc::IN_OPEN) } >= 0
    );
    file
}
fn event(watch: &mut File) -> bool {
    match watch.read(&mut [0; 4096]) {
        Ok(count) => count != 0,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => false,
        Err(error) => panic!("T1147 native observation failed: {error}"),
    }
}
fn release(
    path: PathBuf,
    stop: watch::Sender<bool>,
    ack: mpsc::Receiver<bool>,
    done: mpsc::Receiver<()>,
) -> (bool, bool, std::io::Result<()>) {
    loop {
        match File::options()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(mut file) => {
                stop.send_replace(true);
                let responsive = ack.recv_timeout(Duration::from_secs(3));
                // Independent release on success AND protocol failure; never assert
                // while the original native reader can still own this FIFO.
                let released = file.write_all(b"Blent Front\n");
                return (responsive.is_ok(), responsive.unwrap_or(false), released);
            }
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => {}
            Err(error) => return (false, false, Err(error)),
        }
        if !matches!(done.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            return (false, false, Ok(()));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
