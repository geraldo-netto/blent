//! T815: output budgets apply during collection, not after unlimited spooling.
#![cfg(feature = "commands")]
use blent_config::commands::{AsyncCommandExt, SyncCommandExt};
use std::{
    io::{self, Write},
    process::{Command, Stdio},
    time::Duration,
};

fn fixture(stdout: usize, stderr: usize) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "t815_output_fixture", "--nocapture"])
        .env("BLENT_T815_STDOUT", stdout.to_string())
        .env("BLENT_T815_STDERR", stderr.to_string());
    command
}

#[test]
fn t815_output_fixture() {
    let Ok(stdout) = std::env::var("BLENT_T815_STDOUT") else {
        return;
    };
    let stdout: usize = stdout.parse().unwrap();
    if let Some(path) = std::env::var_os("BLENT_T815_SPAWNED") {
        std::fs::write(path, b"spawned").unwrap();
    }
    let stderr: usize = std::env::var("BLENT_T815_STDERR").unwrap().parse().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = io::stderr().write_all(&vec![b'E'; stderr]);
    });
    let _ = io::stdout().write_all(&vec![b'O'; stdout]);
    writer.join().unwrap();
    // Overflow tests intentionally kill this producer. Keep it out of exit-time
    // coverage flushing until retirement; never race SIGKILL with profile writes.
    if std::env::var_os("BLENT_T837_AWAIT_RETIREMENT").is_some() {
        if let Some(path) = std::env::var_os("BLENT_T837_QUIET_READY") {
            std::fs::write(path, b"ready").unwrap();
        }
        std::thread::sleep(Duration::from_secs(10));
    }
    if std::env::var_os("BLENT_T815_HOLD_INPUT").is_some() {
        std::thread::sleep(Duration::from_secs(1));
    }
    std::process::exit(0);
}

#[test]
fn t815_invalid_limits_reject_before_child_creation() {
    const NAME: &str = "t815_invalid_limits_reject_before_child_creation";
    if std::env::var_os("BLENT_T815_INVALID_LIMIT").is_none() {
        for (name, max) in [
            ("BLENT_COMMAND_STDOUT_BYTES", 67108864u64),
            ("BLENT_COMMAND_STDERR_BYTES", 67108864),
            ("BLENT_COMMAND_TOTAL_BYTES", 134217728),
            ("BLENT_COMMAND_COLLECTORS", 4096),
        ] {
            for bad in [
                "0".to_string(),
                "-1".into(),
                "NaN".into(),
                "1 ".into(),
                (max + 1).to_string(),
                "18446744073709551616".into(),
            ] {
                let output = Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", NAME, "--nocapture"])
                    .env("BLENT_T815_INVALID_LIMIT", "1")
                    .env(name, bad)
                    .output_timeout(Duration::from_secs(5))
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("spawned");
    let mut command = fixture(1, 1);
    command.env("BLENT_T815_SPAWNED", &marker);
    assert_eq!(
        command
            .output_timeout(Duration::from_secs(1))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert_eq!(
        runtime
            .block_on(tokio::process::Command::from(command).output_timeout(Duration::from_secs(1)))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(!marker.exists(), "T815: invalid budget spawned a child");
}

#[test]
fn t815_unrepresentable_deadline_is_rejected_without_panic_or_spawn() {
    for asynchronous in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let marker = root.path().join("spawned");
        let mut command = fixture(1, 1);
        command.env("BLENT_T815_SPAWNED", &marker);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if asynchronous {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(tokio::process::Command::from(command).output_timeout(Duration::MAX))
            } else {
                command.output_timeout(Duration::MAX)
            }
        }));
        assert!(result.is_ok(), "T815: timeout overflow panicked");
        assert_eq!(
            result.unwrap().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!marker.exists());
    }
}

#[test]
fn t837_overflow_fixture_remains_live_until_owned_retirement() {
    let root = tempfile::tempdir().unwrap();
    let ready = root.path().join("quiet");
    let mut child = fixture(1, 1)
        .env("BLENT_T837_AWAIT_RETIREMENT", "1")
        .env("BLENT_T837_QUIET_READY", &ready)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let end = std::time::Instant::now() + Duration::from_secs(2);
    while !ready.exists() && child.try_wait().unwrap().is_none() && std::time::Instant::now() < end
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    let waiting = ready.exists() && child.try_wait().unwrap().is_none();
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        waiting,
        "T837: overflow producer entered process-exit/profile-flush before parent retirement"
    );
}

#[test]
fn t815_overflow_interrupts_blocked_stdin_delivery() {
    if isolated("t815_overflow_interrupts_blocked_stdin_delivery") {
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut command = fixture(262144, 0);
    command.env("BLENT_T815_HOLD_INPUT", "1");
    let started = std::time::Instant::now();
    let error = runtime
        .block_on(
            tokio::process::Command::from(command)
                .output_input_timeout(Some(&vec![7; 4 * 1024 * 1024]), Duration::from_millis(700)),
        )
        .unwrap_err();
    assert_eq!(
        error.kind(),
        io::ErrorKind::InvalidData,
        "T815: blocked input hid output overflow"
    );
    assert!(started.elapsed() < Duration::from_millis(500));
}

fn isolated(name: &str) -> bool {
    if std::env::var_os("BLENT_T815_LIMITS").is_some() {
        return false;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("BLENT_T815_LIMITS", "1")
        .env("BLENT_COMMAND_STDOUT_BYTES", "131072")
        .env("BLENT_COMMAND_STDERR_BYTES", "131072")
        .env("BLENT_COMMAND_TOTAL_BYTES", "196608")
        .output_timeout(Duration::from_secs(15))
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn t815_exact_over_limit_and_simultaneous_streams_are_bounded() {
    if isolated("t815_exact_over_limit_and_simultaneous_streams_are_bounded") {
        return;
    }
    // Account for the native test harness's fixed prelude, not part of payload.
    let prefix = fixture(0, 0).output().unwrap().stdout;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for asynchronous in [false, true] {
        for (stdout, stderr, accepted) in [
            (131072, 0, true),
            (prefix.len(), 131072, true),
            (98304, 98304, true),
            (131073, 0, false),
            (prefix.len(), 131073, false),
            (110000, 110000, false),
        ] {
            let mut command = fixture(stdout - prefix.len(), stderr);
            if !accepted {
                command.env("BLENT_T837_AWAIT_RETIREMENT", "1");
            }
            let result = if asynchronous {
                runtime.block_on(
                    tokio::process::Command::from(command).output_timeout(Duration::from_secs(3)),
                )
            } else {
                command.output_timeout(Duration::from_secs(3))
            };
            if accepted {
                let output = result.unwrap();
                assert!(output.status.success());
                assert_eq!(output.stdout.len(), stdout);
                assert_eq!(output.stderr, vec![b'E'; stderr]);
                assert_eq!(&output.stdout[..prefix.len()], prefix.as_slice());
                assert!(output.stdout[prefix.len()..]
                    .iter()
                    .all(|byte| *byte == b'O'));
            } else {
                assert!(result.is_err_and(|error| error.kind()==io::ErrorKind::InvalidData),
                    "T815: accepted output beyond byte budget: async={asynchronous} stdout={stdout} stderr={stderr}");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn t815_inherited_output_does_not_extend_success_collection() {
    for asynchronous in [false, true] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut command = Command::new("sh");
        command
            .args(["-c", "sleep 0.4 & printf out; printf err >&2; exit 7"])
            .stdin(Stdio::null());
        let started = std::time::Instant::now();
        let output = if asynchronous {
            runtime.block_on(
                tokio::process::Command::from(command).output_timeout(Duration::from_millis(200)),
            )
        } else {
            command.output_timeout(Duration::from_millis(200))
        }
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
        assert!(started.elapsed() < Duration::from_millis(300));
    }
}
