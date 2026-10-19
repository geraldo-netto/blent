//! T1146: rejected endpoints must not establish a controlling-terminal lifetime.
use super::*;
use std::os::unix::fs::OpenOptionsExt;

const TEST: &str =
    "camera::native_tests::terminal::t1146_rejected_private_pty_never_becomes_controlling_terminal";

#[test]
fn t1146_rejected_private_pty_never_becomes_controlling_terminal() {
    if std::env::var_os("BLENT_T1146_CHILD").is_some() {
        rejected();
        return;
    }
    use blent_config::commands::SyncCommandExt;
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env("BLENT_T1146_CHILD", "1")
        .output_bounded()
        .unwrap();
    assert!(
        result.status.success(),
        "T1146 isolated session: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
fn rejected() {
    // The owned command creates a private process group. Leave that group only
    // long enough to become a session leader; no child or resource can escape.
    let parent_group = unsafe { libc::getpgid(libc::getppid()) };
    assert!(parent_group > 0);
    assert_eq!(unsafe { libc::setpgid(0, parent_group) }, 0);
    assert_eq!(unsafe { libc::setsid() }, std::process::id() as i32);
    // Closing this fixture's fresh PTY master may send SIGHUP to this session.
    // The disposition is confined to this disposable, single-test process.
    assert_ne!(
        unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) },
        libc::SIG_ERR
    );
    assert!(
        !controlling_terminal(),
        "T1146 child inherited a controlling terminal"
    );
    let root = tempfile::tempdir().unwrap();
    let (path, master) = device(root.path(), "unused-private-name");
    // No sysfs bind mount: this private PTY is deliberately not a webcam.
    let rejected = outputs::open_device(&path, "Blent Front").is_err();
    let acquired = controlling_terminal();
    drop(master); // Release the actual private endpoint before asserting.
    drop(root);
    assert!(rejected, "T1146 private PTY was accepted as a webcam");
    assert!(
        !acquired,
        "T1146 rejected endpoint established a controlling terminal"
    );
}
fn controlling_terminal() -> bool {
    std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOCTTY | libc::O_CLOEXEC)
        .open("/dev/tty")
        .is_ok()
}
