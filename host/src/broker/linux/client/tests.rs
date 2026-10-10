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
