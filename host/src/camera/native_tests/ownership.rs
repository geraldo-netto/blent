//! T1137: actual writer capability, independent of containing-process lifetime.
use super::*;
use std::io::{Read, Write};

#[test]
fn t1137_writer_keeps_original_device_lease_after_creator_death() {
    isolated("death", "camera::native_tests::ownership::t1137_writer_keeps_original_device_lease_after_creator_death");
}

#[test]
fn t1137_writer_cannot_follow_a_replaced_device_path() {
    isolated(
        "replacement",
        "camera::native_tests::ownership::t1137_writer_cannot_follow_a_replaced_device_path",
    );
}

fn isolated(case: &str, test: &str) {
    if std::env::var_os("BLENT_T1137_ISOLATED").is_some() {
        assert_eq!(
            std::process::id(),
            1,
            "T1137 requires its private PID namespace"
        );
        let result = Command::new("/usr/bin/python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/camera/native_tests/ownership.py"
            ))
            .args(["--case", case, "--binary"])
            .arg(std::env::current_exe().unwrap())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "T1137 {case}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let mut command = Command::new("unshare");
    if unsafe { libc::geteuid() } != 0 {
        command.args(["--user", "--map-root-user"]);
    }
    let result = command
        .args([
            "--mount",
            "--pid",
            "--fork",
            "--mount-proc",
            "--propagation",
            "private",
        ])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("BLENT_T1137_ISOLATED", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "T1137 private namespace: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn t1137_creator_process() {
    let Some(root) = std::env::var_os("BLENT_T1137_ROOT") else {
        return;
    };
    runtime().block_on(creator(PathBuf::from(root)));
}

async fn creator(root: PathBuf) {
    let front = PathBuf::from(std::env::var_os("BLENT_T1137_FRONT").unwrap());
    let mut control = std::os::unix::net::UnixStream::connect(root.join("creator.sock")).unwrap();
    control
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let lease = Arc::new(outputs::open_device(&front, "Blent Front").unwrap());
    control.write_all(b"L").unwrap();
    let mut action = [0];
    control.read_exact(&mut action).unwrap();
    assert_eq!(action, *b"S");
    let options = tests::options();
    let mut frame = vec![16; options.frame_bytes().unwrap()];
    frame[..6].copy_from_slice(b"pixels");
    let (_frames, input) = watch::channel(Some(Arc::new(frame)));
    let report = Report::new(watch::channel(State::Starting).0, watch::channel(None).0);
    let (producer, _original) =
        output_worker::start_with(lease, &options.profile, input, &report, configured).unwrap();
    control.write_all(b"P").unwrap();
    control.set_nonblocking(true).unwrap();
    let mut control = tokio::net::UnixStream::from_std(control).unwrap();
    tokio::io::AsyncReadExt::read_exact(&mut control, &mut action)
        .await
        .unwrap();
    drop(producer);
    report.wait_native().await;
}

#[test]
fn t1137_native_rights_rejections_cannot_retain_file_leases() {
    let result = Command::new("/usr/bin/python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/camera/native_tests/ownership.py"
        ))
        .arg("--protocol-tests")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "T1137 rights: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn configured(
    file: std::fs::File,
    spec: output_device::Spec,
) -> Result<output_device::ConfiguredOutput> {
    use std::os::fd::AsRawFd;
    let root = PathBuf::from(std::env::var_os("BLENT_T1137_ROOT").unwrap());
    let mut endpoint = std::os::unix::net::UnixStream::connect(root.join("writer.sock"))?;
    endpoint.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    // This capability identifies the original process containing the actual
    // blocking writer thread, not a PID selected after creator loss.
    let native = unsafe { libc::syscall(libc::SYS_pidfd_open, std::process::id(), 0) };
    let identity = unsafe { blent_config::linux::packet::owned(native as i32)? };
    blent_config::linux::packet::send(endpoint.as_raw_fd(), b"R", &[identity.as_raw_fd()])?;
    let mut action = [0];
    endpoint.read_exact(&mut action)?;
    anyhow::ensure!(action == *b"W", "T1137 invalid original-writer action");
    output_device::configure_fixture(file, spec)
}
