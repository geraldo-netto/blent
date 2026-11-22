use super::*;
use crate::broker::linux::{async_io::AsyncListener, owner::Owner};
use blent_config::connection::registration::{Message, Rejection};
use std::os::unix::fs::PermissionsExt;

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn launch(root: &std::path::Path, executable: PathBuf) -> Launch {
    Launch {
        executable,
        runtime: root.to_owned(),
        state: root.to_owned(),
        helper: root.join("unused-helper"),
    }
}
fn quote(value: &std::ffi::OsStr) -> String {
    format!("'{}'", value.to_str().unwrap().replace('\'', "'\"'\"'"))
}
fn executable(root: &std::path::Path, count: usize) -> PathBuf {
    let path = root.join("broker-fixture");
    let source = format!("#!/bin/sh\nBLENT_T853_CLIENT_FIXTURE={} BLENT_T853_CLIENT_COUNT={} exec {} --exact broker::linux::client::tests::t853_client_process_fixture --nocapture\n",
        quote(root.as_os_str()), count, quote(std::env::current_exe().unwrap().as_os_str()));
    std::fs::write(&path, source).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

// Invoked as a separate native process by the normal regression below. This
// fixture is not skipped: the parent test controls its environment and deadline.
#[tokio::test]
async fn t853_client_process_fixture() {
    let Some(directory) = std::env::var_os("BLENT_T853_CLIENT_FIXTURE") else {
        return;
    };
    let count: usize = std::env::var("BLENT_T853_CLIENT_COUNT")
        .unwrap()
        .parse()
        .unwrap();
    let owner = match Owner::bind(std::path::Path::new(&directory), 8) {
        Ok(owner) => owner,
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
        Err(error) => panic!("fixture election failed: {error}"),
    };
    let listener = AsyncListener::new(owner).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        for _ in 0..count {
            let (socket, _) = listener.accept().await.unwrap();
            let request = socket.receive().await.unwrap().message.request();
            socket
                .send(
                    &Message::Rejected {
                        request,
                        reason: Rejection::Unavailable,
                    },
                    &[],
                )
                .await
                .unwrap();
        }
    })
    .await
    .unwrap();
}
async fn exchange(socket: AsyncSocket, request: u64) {
    socket
        .send(
            &Message::Rejected {
                request,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
        .unwrap();
    assert!(matches!(socket.receive().await.unwrap().message,
        Message::Rejected { request: value, reason: Rejection::Unavailable } if value == request));
}

#[tokio::test]
async fn t853_competing_clients_start_one_elected_owner_and_exchange() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let root = root();
        let connector = Connector::new(launch(root.path(), executable(root.path(), 2)), 2).unwrap();
        let (first, second) = tokio::join!(
            connector.connect(Duration::from_secs(3)),
            connector.connect(Duration::from_secs(3))
        );
        tokio::join!(exchange(first.unwrap(), 1), exchange(second.unwrap(), 2));
        assert_eq!(connector.starters.available_permits(), 2);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t853_existing_owner_needs_no_executable_and_invalid_inputs_fail() {
    let root = root();
    let owner = Owner::bind(root.path(), 2).unwrap();
    let connector = Connector::new(launch(root.path(), root.path().join("absent")), 1).unwrap();
    let client = connector.connect(Duration::from_secs(1)).await.unwrap();
    assert_eq!(connector.starters.available_permits(), 1);
    drop(client);
    drop(owner);
    for timeout in [Duration::ZERO, Duration::from_secs(61)] {
        assert!(connector.connect(timeout).await.is_err());
    }
    assert!(connector.connect(Duration::from_secs(1)).await.is_err());
    for capacity in [0, 65, usize::MAX] {
        assert!(Connector::new(launch(root.path(), "/bin/false".into()), capacity).is_err());
    }
    assert!(Connector::new(launch(root.path(), "relative".into()), 1).is_err());
}

#[tokio::test]
async fn t853_unavailable_or_busy_owner_has_one_absolute_startup_deadline() {
    let root = root();
    let connector = Connector::new(launch(root.path(), "/bin/true".into()), 1).unwrap();
    assert!(connector.connect(Duration::from_millis(100)).await.is_err());
    assert_eq!(connector.starters.available_permits(), 1);
    let permit = connector.starters.clone().acquire_owned().await.unwrap();
    assert!(connector.connect(Duration::from_millis(10)).await.is_err());
    assert_eq!(connector.starters.available_permits(), 0);
    drop(permit);
    assert!(missing(&io::Error::from(io::ErrorKind::ConnectionRefused)));
    assert!(missing(&io::Error::from(io::ErrorKind::NotFound)));
    assert!(!missing(&io::Error::from(io::ErrorKind::PermissionDenied)));
    assert!(busy(&io::Error::from(io::ErrorKind::WouldBlock)));
    assert!(!busy(&io::Error::from(io::ErrorKind::PermissionDenied)));
}

#[tokio::test]
async fn t853_invalid_socket_path_fails_without_losing_starter_capacity() {
    let root = root();
    std::fs::create_dir(root.path().join("broker.sock")).unwrap();
    let connector = Connector::new(launch(root.path(), root.path().join("absent")), 1).unwrap();
    assert!(connector.connect(Duration::from_millis(100)).await.is_err());
    assert_eq!(connector.starters.available_permits(), 1);
}

#[tokio::test]
async fn t853_timed_out_native_starter_retains_capacity_until_actual_return() {
    let root = root();
    let connector = Connector::new(launch(root.path(), "/bin/true".into()), 1).unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let (entered, entry) = tokio::sync::oneshot::channel();
    let start = connector.start_with(Instant::now() + Duration::from_millis(50), move || {
        entered.send(()).unwrap();
        blocked.recv_timeout(Duration::from_secs(5)).unwrap();
        Ok(())
    });
    let (result, entered) = tokio::join!(start, entry);
    entered.unwrap();
    assert!(result.is_err());
    assert_eq!(connector.starters.available_permits(), 0);
    assert!(connector.connect(Duration::from_millis(10)).await.is_err());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        let _permit = connector.starters.acquire().await.unwrap();
    })
    .await
    .unwrap();
    assert_eq!(connector.starters.available_permits(), 1);
}

#[test]
fn t1473_shared_broker_launch_leaves_callers_cgroup() {
    let root = root();
    let script = root.path().join("broker-placement");
    let report = root.path().join("group");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n/bin/cat /proc/self/cgroup > {}\n",
            quote(report.as_os_str())
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    launch(root.path(), script).spawn().unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let actual = loop {
        if let Ok(text) = std::fs::read_to_string(&report) {
            if !text.is_empty() {
                break text;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "T1473 broker did not publish placement"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_ne!(
        actual,
        std::fs::read_to_string("/proc/self/cgroup").unwrap(),
        "T1473 shared broker inherited client retirement group"
    );
    assert!(actual.contains(".scope"));
}

#[tokio::test]
async fn t1638_failed_broker_reports_exit_before_startup_deadline() {
    let root = root();
    let program = root.path().join("rejected-broker");
    std::fs::write(&program, b"#!/bin/sh\nexit 7\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let connector = Connector::new(launch(root.path(), program), 1).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        connector.connect(Duration::from_secs(10)),
    )
    .await
    .expect("T1638 exited broker still consumes the full startup deadline");
    let failure = result
        .err()
        .expect("T1638 failed broker was connected")
        .to_string();
    assert!(
        failure.contains("7"),
        "T1638 original exit status missing: {failure}"
    );
    assert_eq!(connector.starters.available_permits(), 1);
}

#[tokio::test]
async fn t1638_exited_starter_waits_for_the_other_elected_owner() {
    use std::os::unix::fs::OpenOptionsExt;
    let root = root();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .open(root.path().join("broker.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    let mut child =
        blent_config::commands::spawn_reaped(&mut std::process::Command::new("/bin/false"))
            .unwrap();
    tokio::time::timeout(Duration::from_secs(1), child.exit.wait_for(Option::is_some))
        .await
        .unwrap()
        .unwrap();
    let connector = Connector::new(launch(root.path(), "/bin/false".into()), 1).unwrap();
    let connection = connector.wait(Instant::now() + Duration::from_secs(2), Some(child));
    tokio::pin!(connection);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut connection)
            .await
            .is_err(),
        "T1638 losing starter discarded an initializing owner"
    );
    drop(lock);
    let owner = Owner::bind(root.path(), 2).unwrap();
    let socket = connection.await.unwrap();
    drop(socket);
    drop(owner);
}

#[tokio::test]
async fn t1638_dead_starter_without_runtime_reports_original_status() {
    let root = root();
    let mut child =
        blent_config::commands::spawn_reaped(&mut std::process::Command::new("/bin/false"))
            .unwrap();
    tokio::time::timeout(Duration::from_secs(1), child.exit.wait_for(Option::is_some))
        .await
        .unwrap()
        .unwrap();
    let connector =
        Connector::new(launch(&root.path().join("absent"), "/bin/false".into()), 1).unwrap();
    let result = connector
        .wait(Instant::now() + Duration::from_secs(10), Some(child))
        .await;
    assert!(result.err().unwrap().to_string().contains("exit status: 1"));
    assert!(connector.check_starter(None).is_ok());
}

#[test]
fn t1600_launch_passes_validated_capacity_settings() {
    if crate::test_logging::isolated(
        "broker::linux::client::tests::t1600_launch_passes_validated_capacity_settings",
    ) {
        return;
    }
    for name in [
        "BLENT_USB_CLIENTS",
        "BLENT_USB_LINKS",
        "BLENT_USB_REGISTRATIONS",
    ] {
        std::env::remove_var(name);
    }
    let launcher = launch(std::path::Path::new("/private"), "/bin/false".into());
    let args: Vec<_> = launcher
        .command()
        .unwrap()
        .get_args()
        .map(|arg| arg.to_owned())
        .collect();
    assert!(args.ends_with(
        &["--clients", "128", "--links", "4", "--registrations", "64"]
            .map(std::ffi::OsString::from)
    ));
    for (name, max) in [
        ("BLENT_USB_CLIENTS", 1024),
        ("BLENT_USB_LINKS", 64),
        ("BLENT_USB_REGISTRATIONS", 1024),
    ] {
        capacity_values(&launcher, name, max);
    }
}

fn capacity_values(launcher: &Launch, name: &str, max: usize) {
    for invalid in [
        "0".to_owned(),
        "-1".into(),
        "invalid".into(),
        (max + 1).to_string(),
    ] {
        std::env::set_var(name, invalid);
        assert!(launcher.command().is_err());
    }
    for value in [1, max] {
        std::env::set_var(name, value.to_string());
        let command = launcher.command().unwrap();
        assert!(command
            .get_args()
            .any(|arg| arg == value.to_string().as_str()));
    }
    std::env::remove_var(name);
}
