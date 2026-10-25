//! T572: native open/lock/lifecycle coverage in a private mount namespace.
use super::*;
use std::{ffi::CStr, os::fd::FromRawFd, os::unix::fs::MetadataExt, process::Command};

mod admission;
mod ownership;
mod retirement;
mod startup;
mod terminal;
mod virtual_output;

const TEST: &str = "camera::native_tests::t572_isolated_native_devices_and_session_cleanup";

#[test]
fn t572_isolated_native_devices_and_session_cleanup() {
    if private(TEST) {
        fixture(|root, front, rear| runtime().block_on(lifecycle(root, front, rear)));
    }
}

fn private(test: &str) -> bool {
    if std::env::var_os("BLENT_T572_MOUNTS").is_some() {
        return true;
    }
    let mut command = Command::new("unshare");
    if unsafe { libc::geteuid() } != 0 {
        command.args(["--user", "--map-root-user"]);
    }
    let result = command
        .args(["--mount", "--fork", "--propagation", "private"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("BLENT_T572_MOUNTS", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "Private camera fixture {test}: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    false
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn device(root: &std::path::Path, label: &str) -> (PathBuf, std::fs::File) {
    // Fresh PTYs are character devices owned only by this fixture. No root
    // mknod capability or live webcam node is needed.
    let fd = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC) };
    assert!(fd >= 0, "{}", std::io::Error::last_os_error());
    let master = unsafe { std::fs::File::from_raw_fd(fd) };
    assert_eq!(unsafe { libc::grantpt(fd) }, 0);
    assert_eq!(unsafe { libc::unlockpt(fd) }, 0);
    let mut name = [0; 1024];
    assert_eq!(
        unsafe { libc::ptsname_r(fd, name.as_mut_ptr(), name.len()) },
        0
    );
    let path = PathBuf::from(unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap());
    let metadata = std::fs::metadata(&path).unwrap();
    let sys = root.join(format!(
        "sys/{}:{}",
        libc::major(metadata.rdev()),
        libc::minor(metadata.rdev())
    ));
    std::fs::create_dir_all(&sys).unwrap();
    std::fs::write(sys.join("name"), label).unwrap();
    (path, master)
}

fn fixture(action: impl FnOnce(&std::path::Path, PathBuf, PathBuf)) {
    let root = tempfile::tempdir().unwrap();
    let (front, _front_owner) = device(root.path(), "Blent Front\n");
    let (rear, _rear_owner) = device(root.path(), "Blent Rear\n");
    assert!(Command::new("mount")
        .args(["--bind"])
        .arg(root.path().join("sys"))
        .arg("/sys/dev/char")
        .status()
        .unwrap()
        .success());
    let file = outputs::open_device(&front, "Blent Front").unwrap();
    assert!(outputs::open_device(&front, "Blent Front")
        .unwrap_err()
        .to_string()
        .contains("already owned"));
    assert!(outputs::open_device(&rear, "Blent Front")
        .unwrap_err()
        .to_string()
        .contains("labelled"));
    let link = root.path().join("symlink");
    std::os::unix::fs::symlink(&front, &link).unwrap();
    assert!(outputs::open_device(&link, "Blent Front").is_err());
    drop(file);
    action(root.path(), front, rear);
}

async fn lifecycle(root: &std::path::Path, front: PathBuf, rear: PathBuf) {
    tests::script(root, "ffmpeg", "import sys\nsys.exit(1)");
    tests::script(
        root,
        "adb",
        &format!(
            r#"import sys, pathlib
root=pathlib.Path({root:?})
a=sys.argv
if '--no-rebind' in a:
    (root/'mapping').write_text(a[-2]+' '+a[-1])
elif '--list' in a:
    if (root/'mapping').exists(): print('Usb '+(root/'mapping').read_text())
elif '--remove' in a: (root/'mapping').unlink()
elif 'shell' in a and 'broadcast' in sys.stdin.read(): print('Broadcast completed: result=1')
else: print('fixture-tablet')"#
        ),
    );
    // This process is a single-test child; no sibling tests see its PATH.
    std::env::set_var("PATH", root);
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    for stopped in [true, false] {
        let (_stop, stop) = watch::channel(stopped);
        let status = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            run_native(options.clone(), stop, status),
        )
        .await
        .expect("T572 bounded native session");
        assert_eq!(result.is_ok(), stopped, "{result:?}");
        assert!(
            !root.join("mapping").exists(),
            "T572 reverse mapping leaked"
        );
        // Both leases must retire after graceful stop and producer failure.
        drop(outputs::open_device(&options.front_device, "Blent Front").unwrap());
        drop(outputs::open_device(&options.rear_device, "Blent Rear").unwrap());
    }
}

#[test]
fn t1088_logical_camera_success_cannot_hide_original_cleanup_failure() {
    if private(
        "camera::native_tests::t1088_logical_camera_success_cannot_hide_original_cleanup_failure",
    ) {
        fixture(|root, front, rear| runtime().block_on(cleanup_failure(root, front, rear)));
    }
}
async fn cleanup_failure(root: &std::path::Path, front: PathBuf, rear: PathBuf) {
    tests::script(root, "ffmpeg", "import sys\nsys.exit(1)");
    tests::script(
        root,
        "adb",
        &format!(
            r#"import pathlib, sys
root = pathlib.Path({root:?})
args = sys.argv[1:]
if '--no-rebind' in args:
    import fcntl
    (root/'mapping').write_text(args[-2] + ' ' + args[-1])
    with (root/'stop-gate').open() as gate: fcntl.flock(gate, fcntl.LOCK_SH)
elif '--list' in args:
    if (root/'mapping').exists():
        if not (root/'recover').exists():
            (root/'cleanup-failed').touch()
            print('T1088 original cleanup unavailable', file=sys.stderr); sys.exit(7)
        print('Usb ' + (root/'mapping').read_text())
elif '--remove' in args:
    (root/'mapping').unlink()
else:
    print('fixture-tablet')
"#
        ),
    );
    std::env::set_var("PATH", root);
    let mut options = tests::options();
    options.front_device = front;
    options.rear_device = rear;
    let gate = std::fs::File::create(root.join("stop-gate")).unwrap();
    use std::os::fd::AsRawFd;
    assert_eq!(unsafe { libc::flock(gate.as_raw_fd(), libc::LOCK_EX) }, 0);
    let (stop, stopped) = watch::channel(false);
    let report = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
    let done = std::sync::atomic::AtomicBool::new(false);
    let running = async {
        let result = run_native(options.clone(), stopped, report.clone()).await;
        done.store(true, std::sync::atomic::Ordering::Release);
        result
    };
    // T1147 rejects pre-stopped startup without creating anything. Preserve the
    // T1088 cleanup oracle after a connection was actually admitted instead.
    let stop_after_mapping = async {
        while !root.join("mapping").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        stop.send_replace(true);
        drop(gate);
        while !root.join("cleanup-failed").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert!(
            root.join("mapping").exists(),
            "T1088 failed mapping was released early"
        );
        assert!(!done.load(std::sync::atomic::Ordering::Acquire));
        assert!(report
            .reserve_native(control::NativeResource::Connection)
            .is_err());
        std::fs::write(root.join("recover"), "").unwrap();
    };
    let (result, ()) = tokio::join!(running, stop_after_mapping);
    assert!(
        !root.join("mapping").exists(),
        "T1088 original mapping was not actually retired"
    );
    // The private fake mapping is gone only after the original retained worker
    // actually cleans it. Verify device leases after that fence as well.
    drop(outputs::open_device(&options.front_device, "Blent Front").unwrap());
    drop(outputs::open_device(&options.rear_device, "Blent Rear").unwrap());
    let error = result.expect_err("T1088 camera reported success after original cleanup failed");
    assert!(format!("{error:#}").contains("T1088 original cleanup unavailable"));
}
