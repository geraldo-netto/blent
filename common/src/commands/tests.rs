use super::*;
use std::io::Read;

// T496: re-exec the native test executable; no shell or external sleep program.
pub(super) fn fixture(path: &std::path::Path, role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "commands::tests::t496_fixture_process",
            "--nocapture",
        ])
        .env("BLENT_T496_PATH", path)
        .env("BLENT_T496_ROLE", role);
    command
}

#[test]
fn t496_fixture_process() {
    let Some(raw_path) = std::env::var_os("BLENT_T496_PATH") else {
        return;
    };
    let path = std::path::PathBuf::from(raw_path);
    let stage = path.with_extension("tmp");
    std::fs::write(&stage, std::process::id().to_string()).unwrap();
    std::fs::rename(stage, &path).unwrap();
    let role = std::env::var("BLENT_T496_ROLE").unwrap();
    if role == "input-delayed" {
        std::thread::sleep(Duration::from_millis(350));
    }
    match role.as_str() {
        "input" | "input-delayed" => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            std::fs::write(path.with_extension("input"), bytes).unwrap();
        }
        "denied" => std::thread::sleep(Duration::from_millis(400)),
        "wait" => std::thread::sleep(Duration::from_secs(10)),
        _ => panic!("T496: unknown fixture role"),
    }
    std::fs::write(path.with_extension("done"), "done").unwrap();
}

pub(super) fn assert_exited(pid: u32) {
    #[cfg(target_os = "linux")]
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    #[cfg(windows)]
    {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::*};
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if !handle.is_null() {
            let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
            assert_eq!(
                unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
                WAIT_OBJECT_0
            );
        }
    }
}

fn fixture_timeout() -> Duration {
    // Keep the original Linux deadline; allow native executable startup on Windows.
    if cfg!(windows) {
        Duration::from_millis(500)
    } else {
        Duration::from_millis(50)
    }
}

#[test]
fn t328_denied_signal_does_not_extend_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pid");
    let child = fixture(&path, "denied").spawn().unwrap();
    let pid = child.id();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "T496: fixture did not start"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let start = std::time::Instant::now();
    // Isolate the EPERM result of a privileged child without authorizing
    // real system setup or depending on root in the normal test suite.
    reap_after_signal(child, Err(io::Error::from(io::ErrorKind::PermissionDenied)));
    let elapsed = start.elapsed();
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        path.with_extension("done").exists(),
        "T328: denied work can continue after the deadline"
    );
    assert_exited(pid);
    assert!(
        elapsed < Duration::from_millis(200),
        "T328: denied cancellation blocked for {elapsed:?}"
    );
    assert!(timed_out().to_string().contains("may still be running"));
}

#[test]
fn t268_lifecycle_deadlines_cover_cli_and_shipped_service_budgets() {
    assert!(daemon_command_timeout(false) > DAEMON_STOP_TIMEOUT);
    assert!(SERVICE_STOP_TIMEOUT > DAEMON_STOP_TIMEOUT);
    assert!(daemon_command_timeout(true) > SERVICE_STOP_TIMEOUT * 2);
    for unit in [
        include_str!("../../../scripts/blent.service"),
        include_str!("../../../packaging/blent.service"),
    ] {
        let seconds: u64 = unit
            .lines()
            .find_map(|line| line.strip_prefix("TimeoutStopSec="))
            .expect("T268: explicit service stop deadline")
            .parse()
            .unwrap();
        assert_eq!(Duration::from_secs(seconds), SERVICE_STOP_TIMEOUT);
    }
}

fn assert_reaped(path: &std::path::Path) {
    let pid = std::fs::read_to_string(path).unwrap();
    assert_exited(pid.trim().parse().unwrap());
}

// T094: subprocess deadlines must kill and reap, not abandon hung commands.
#[test]
fn t094_sync_command_is_bounded_and_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pid");
    let start = std::time::Instant::now();
    let result = fixture(&path, "wait").output_timeout(fixture_timeout());
    assert!(result.is_err_and(|e| e.kind() == io::ErrorKind::TimedOut));
    assert!(start.elapsed() < Duration::from_millis(800));
    assert_reaped(&path);
}

#[tokio::test]
async fn t094_async_command_is_bounded_and_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pid");
    let result = tokio::process::Command::from(fixture(&path, "wait"))
        .output_timeout(fixture_timeout())
        .await;
    assert!(result.is_err_and(|e| e.kind() == io::ErrorKind::TimedOut));
    assert_reaped(&path);
}

#[tokio::test]
async fn t094_cancelled_command_is_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pid");
    let task_path = path.clone();
    let task = tokio::spawn(async move {
        tokio::process::Command::from(fixture(&task_path, "wait"))
            .output_bounded()
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !path.exists() {
            assert!(
                !task.is_finished(),
                "T496: fixture failed before publishing its PID"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("T496: fixture readiness deadline");
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_reaped(&path);
}

#[cfg(target_os = "linux")]
fn deny_kill_in_test_thread() {
    // T590: emulate EPERM without signalling any real process or changing UID.
    // The filter applies only to this isolated child's test thread.
    let mut instructions = [
        libc::sock_filter {
            code: (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16,
            jt: 0,
            jf: 0,
            k: 0,
        },
        libc::sock_filter {
            code: (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16,
            jt: 0,
            jf: 1,
            k: libc::SYS_kill as u32,
        },
        libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ERRNO | libc::EPERM as u32,
        },
        libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ALLOW,
        },
    ];
    let filter = libc::sock_fprog {
        len: instructions.len() as u16,
        filter: instructions.as_mut_ptr(),
    };
    unsafe {
        assert_eq!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0), 0);
        assert_eq!(
            libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &filter),
            0
        );
        assert_eq!(libc::kill(std::process::id() as libc::pid_t, 0), -1);
    }
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::EPERM));
}

#[cfg(target_os = "linux")]
#[test]
fn t590_denied_group_signal_reports_unretired_work() {
    const NAME: &str = "commands::tests::t590_denied_group_signal_reports_unretired_work";
    if std::env::var_os("BLENT_T590_DENIED_SIGNAL").is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("BLENT_T590_DENIED_SIGNAL", "1")
            .output_timeout(Duration::from_secs(10))
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(log.reopen().unwrap())
        .finish();
    deny_kill_in_test_thread();
    tracing::subscriber::with_default(subscriber, || terminate_group(std::process::id()));
    assert!(std::fs::read_to_string(log.path())
        .unwrap()
        .contains("Command group could not be terminated; delegated work may continue"));
}

#[cfg(target_os = "linux")]
#[test]
fn t815_collection_failure_after_reap_never_signals_a_reusable_pid() {
    const NAME: &str =
        "commands::tests::t815_collection_failure_after_reap_never_signals_a_reusable_pid";
    if std::env::var_os("BLENT_T815_REAP_SIGNAL").is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("BLENT_T815_REAP_SIGNAL", "1")
            .output_timeout(Duration::from_secs(5))
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(log.reopen().unwrap())
        .finish();
    let root = tempfile::tempdir().unwrap();
    let mut child = fixture(&root.path().join("pid"), "input")
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    child.wait().unwrap();
    let admission = retirement::reserve().unwrap();
    deny_kill_in_test_thread();
    tracing::subscriber::with_default(subscriber, || retire_sync(child, admission));
    assert!(
        !std::fs::read_to_string(log.path())
            .unwrap()
            .contains("could not be terminated"),
        "T815: attempted group signal after direct child was already reaped"
    );
}

#[tokio::test]
async fn t609_stdin_delivery_closes_input_and_preserves_bytes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pid");
    let input = b"secret ' \" \n\0 end";
    let output = tokio::process::Command::from(fixture(&path, "input"))
        .output_input_timeout(Some(input), Duration::from_secs(2))
        .await
        .unwrap();
    assert!(output.status.success());
    assert_eq!(std::fs::read(path.with_extension("input")).unwrap(), input);
    assert_reaped(&path);
}

#[tokio::test]
async fn t609_timeout_covers_blocked_stdin_write() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pid");
    let input = vec![0; 1024 * 1024];
    let error = tokio::process::Command::from(fixture(&path, "wait"))
        .output_input_timeout(Some(&input), fixture_timeout())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert_reaped(&path);
}

#[tokio::test]
async fn t609_cancelled_stdin_writer_is_reaped() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("pid");
    let task_path = path.clone();
    let task = tokio::spawn(async move {
        tokio::process::Command::from(fixture(&task_path, "wait"))
            .output_input_timeout(Some(&vec![0; 1024 * 1024]), Duration::from_secs(10))
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_reaped(&path);
}

#[test]
fn t814_accepted_signal_does_not_make_reaping_unbounded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pid");
    let child = fixture(&path, "denied").spawn().unwrap();
    let pid = child.id();
    let start = std::time::Instant::now();
    // A successful signal request does not prove completion. The fixture stays
    // alive for 400 ms; unlike a truly unkillable process it always self-retires.
    reap_after_signal(child, Ok(()));
    let elapsed = start.elapsed();
    std::thread::sleep(Duration::from_millis(500));
    assert_exited(pid);
    assert!(
        elapsed < Duration::from_millis(200),
        "T814: successful signal caused an unbounded wait: {elapsed:?}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn t814_unconfirmed_async_exit_retains_admission_after_control_deadline() {
    const NAME: &str =
        "commands::tests::t814_unconfirmed_async_exit_retains_admission_after_control_deadline";
    if std::env::var_os("BLENT_T814_HELD_REAP").is_none() {
        for denied in [false, true] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME, "--nocapture"])
                .env(
                    "BLENT_T814_HELD_REAP",
                    if denied { "denied" } else { "accepted" },
                )
                .env("BLENT_PROCESS_LIMIT", "1")
                .env("BLENT_REAP_TIMEOUT_MS", "20")
                .output_timeout(Duration::from_secs(5))
                .unwrap();
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    let denied = std::env::var("BLENT_T814_HELD_REAP").unwrap() == "denied";
    // This disposable process makes kill(2) fail or report success without sending
    // a signal. The owned fixture always exits itself after 400 ms.
    let mut instructions = [
        libc::sock_filter {
            code: (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16,
            jt: 0,
            jf: 0,
            k: 0,
        },
        libc::sock_filter {
            code: (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16,
            jt: 0,
            jf: 1,
            k: libc::SYS_kill as u32,
        },
        libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ERRNO | if denied { libc::EPERM as u32 } else { 0 },
        },
        libc::sock_filter {
            code: (libc::BPF_RET | libc::BPF_K) as u16,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ALLOW,
        },
    ];
    let filter = libc::sock_fprog {
        len: instructions.len() as u16,
        filter: instructions.as_mut_ptr(),
    };
    unsafe {
        assert_eq!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0), 0);
        assert_eq!(
            libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &filter),
            0
        );
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("pid");
            let child =
                OwnedChild::spawn(&mut tokio::process::Command::from(fixture(&path, "denied")))
                    .unwrap();
            let pid = child.id().unwrap();
            let report = child.retire(Duration::ZERO).await;
            assert_eq!(report.phase, RetirementPhase::Incomplete);
            assert_eq!(
                matches!(report.error, Some(RetirementError::Signal(_))),
                denied
            );
            assert!(!report.completion.is_complete());
            assert_eq!(
                OwnedChild::spawn(&mut tokio::process::Command::from(fixture(&path, "denied")))
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::WouldBlock
            );
            tokio::time::timeout(Duration::from_secs(2), async {
                while !report.completion.is_complete() {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            assert_exited(pid);
            assert_readmitted_retirement(&path).await;
        });
}

#[test]
fn t814_dependent_lease_cleanup_cannot_hold_control_deadline() {
    struct Held {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl Drop for Held {
        fn drop(&mut self) {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
        }
    }
    let (started, entered) = std::sync::mpsc::channel();
    let (release, hold) = std::sync::mpsc::channel();
    let (returned, finished) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let dir = tempfile::tempdir().unwrap();
                let mut child = OwnedChild::spawn(&mut tokio::process::Command::from(fixture(
                    &dir.path().join("pid"),
                    "input",
                )))
                .unwrap();
                child.keep_alive(Held {
                    entered: started,
                    release: std::sync::Mutex::new(hold),
                });
                returned
                    .send(child.retire(Duration::from_millis(100)).await)
                    .unwrap();
            });
    });
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    let result = finished.recv_timeout(Duration::from_millis(500));
    let pending = result.as_ref().is_ok_and(|report| {
        report.phase == RetirementPhase::Incomplete && !report.completion.is_complete()
    });
    release.send(()).unwrap();
    worker.join().unwrap();
    assert!(
        result.is_ok(),
        "T814: dependent lease destructor held control shutdown"
    );
    assert!(pending, "T814: live dependent resource reported complete");
    let report = result.unwrap();
    let end = std::time::Instant::now() + Duration::from_secs(2);
    while !report.completion.is_complete() {
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn t836_extreme_retirement_grace_never_panics_or_loses_ownership() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    struct Lease(Arc<AtomicBool>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    for grace in [Duration::MAX, Duration::from_secs(u64::MAX / 2)] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let mut child = runtime.block_on(async {
            OwnedChild::spawn(&mut tokio::process::Command::from(fixture(
                &root.path().join("pid"),
                "denied",
            )))
            .unwrap()
        });
        let pid = child.id().unwrap();
        t836_fixture_ready(&root.path().join("pid"));
        child.keep_alive(Lease(released.clone()));
        let ticket = child.completion();
        let started = std::time::Instant::now();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(child.retire(grace))
        }));
        let duration = started.elapsed();
        let end = std::time::Instant::now() + Duration::from_secs(2);
        while !ticket.is_complete() {
            assert!(std::time::Instant::now() < end);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(released.load(Ordering::Acquire));
        assert_exited(pid);
        assert!(duration < Duration::from_millis(200));
        assert!(
            result.is_ok(),
            "T836: invalid grace panicked during owned retirement"
        );
        let report = result.unwrap();
        assert_eq!(report.phase, RetirementPhase::Incomplete);
        assert_eq!(report.error, Some(RetirementError::InvalidGrace));
    }
}

#[tokio::test]
async fn t836_representable_grace_and_cancelled_retirement_preserve_ownership() {
    for grace in [
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_millis(1),
        Duration::from_secs(1_000_000_000),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut command = tokio::process::Command::from(fixture(&root.path().join("pid"), "input"));
        command.stdin(Stdio::null());
        let child = OwnedChild::spawn(&mut command).unwrap();
        t836_fixture_ready(&root.path().join("pid"));
        let native: &tokio::process::Child = &child;
        assert_eq!(native.id(), child.id());
        let report = child.retire(grace).await;
        assert!(report.error.is_none());
        assert!(report.completion.is_complete());
        assert!(matches!(
            report.phase,
            RetirementPhase::Exited | RetirementPhase::ReapedAfterSignal
        ));
    }
    for grace in [Duration::from_secs(60), Duration::MAX] {
        let root = tempfile::tempdir().unwrap();
        let child = OwnedChild::spawn(&mut tokio::process::Command::from(fixture(
            &root.path().join("pid"),
            "denied",
        )))
        .unwrap();
        let ticket = child.completion();
        let pid = child.id().unwrap();
        t836_fixture_ready(&root.path().join("pid"));
        let task = tokio::spawn(child.retire(grace));
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(2), async {
            while !ticket.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_exited(pid);
    }
}

fn t836_fixture_ready(path: &std::path::Path) {
    let end = std::time::Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(
            std::time::Instant::now() < end,
            "T836 fixture did not start"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(target_os = "linux")]
async fn readmitted_retirement(mut child: OwnedChild) -> Retirement {
    // Readmission proves capacity recovery, not a bound on process startup/exit.
    // Observe this original child's exit before asserting its exited phase.
    assert!(child.wait().await.unwrap().success());
    child.retire(Duration::ZERO).await
}

#[cfg(target_os = "linux")]
fn readmitted_fixture(
    path: &std::path::Path,
) -> (OwnedChild, Option<std::os::unix::net::UnixStream>) {
    use std::{os::fd::OwnedFd, os::unix::net::UnixStream};
    let mut command = fixture(path, "denied");
    let gate = if std::env::var_os("BLENT_T1197_CAUSAL_GATE").is_some() {
        let (held, receiver) = UnixStream::pair().unwrap();
        command = fixture(path, "input");
        command.stdin(Stdio::from(OwnedFd::from(receiver)));
        Some(held)
    } else {
        None
    };
    (
        OwnedChild::spawn(&mut tokio::process::Command::from(command)).unwrap(),
        gate,
    )
}

#[cfg(target_os = "linux")]
async fn assert_readmitted_retirement(path: &std::path::Path) {
    let (child, gate) = readmitted_fixture(path);
    let pid = child.id().unwrap();
    let mut settling = Box::pin(readmitted_retirement(child));
    let early = if gate.is_some() {
        tokio::select! {
            report = settling.as_mut() => Some(report),
            // Exceed the original one-second grace plus 20ms reap contract.
            _ = tokio::time::sleep(Duration::from_millis(1100)) => None,
        }
    } else {
        None
    };
    let returned_before_release = early.is_some();
    // The parent owns the sole writer. EOF also releases on panic/owner loss.
    drop(gate);
    let report = match early {
        Some(report) => report,
        None => settling.await,
    };
    while !report.completion.is_complete() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_exited(pid);
    assert!(
        !returned_before_release,
        "T1197 native exit inferred before the fixture gate released"
    );
    assert_eq!(report.phase, RetirementPhase::Exited);
    assert!(report.completion.is_complete());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn t1197_readmitted_fixture_waits_for_actual_exit_without_a_startup_deadline() {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "commands::tests::t814_unconfirmed_async_exit_retains_admission_after_control_deadline",
            "--nocapture",
        ])
        .env("BLENT_T1197_CAUSAL_GATE", "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = OwnedChild::spawn(&mut command).unwrap();
    let status = child.wait().await.unwrap();
    let retired = child.retire(Duration::ZERO).await;
    assert!(retired.completion.is_complete());
    assert!(
        status.success(),
        "T1197 held native fixture reproduced the one-second phase assumption"
    );
}
