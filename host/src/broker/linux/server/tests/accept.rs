//! T1595: a native accept error must not terminate service for existing clients.
use super::*;

struct FileLimit(libc::rlimit);
impl FileLimit {
    fn exhausted() -> Self {
        let mut original = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        assert_eq!(
            unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut original) },
            0
        );
        let limited = libc::rlimit {
            rlim_cur: 0,
            rlim_max: original.rlim_max,
        };
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limited) }, 0);
        Self(original)
    }
}
impl Drop for FileLimit {
    fn drop(&mut self) {
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &self.0) }, 0);
    }
}

#[tokio::test]
async fn t1595_native_accept_exhaustion_keeps_existing_clients_served() {
    if crate::test_logging::isolated("broker::linux::server::tests::accept::t1595_native_accept_exhaustion_keeps_existing_clients_served") { return; }
    let mut fixture = Fixture::new().await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(60);
    let (client, remote) = fixture.client().await;
    let pending = Socket::connect(
        &fixture.server.settings.runtime.join("broker.sock"),
        unsafe { libc::geteuid() },
    )
    .unwrap();
    remote
        .send(
            &Message::Rejected {
                request: 1,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
        .unwrap();
    let limit = FileLimit::exhausted();
    let (running, response) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(200), fixture.server.run()),
        tokio::time::timeout(Duration::from_millis(150), remote.receive())
    );
    drop(limit);
    let retained = fixture.server.clients.contains_key(&client);
    let replacement = AsyncSocket::new(pending).unwrap();
    replacement
        .send(
            &Message::Rejected {
                request: 2,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
        .unwrap();
    let (recovered, new_response) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(200), fixture.server.run()),
        tokio::time::timeout(Duration::from_millis(150), replacement.receive())
    );
    replacement.close();
    remote.close();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
    assert!(
        running.is_err(),
        "T1595 accept error terminated the broker: {running:?}"
    );
    assert!(matches!(
        response.unwrap().unwrap().message,
        Message::Rejected { request: 1, .. }
    ));
    assert!(retained, "T1595 unrelated client lost its owner");
    assert!(
        recovered.is_err(),
        "T1595 broker terminated after capacity returned"
    );
    assert!(matches!(
        new_response.unwrap().unwrap().message,
        Message::Rejected { request: 2, .. }
    ));
}

#[tokio::test]
async fn t1595_rejected_native_credentials_preserve_existing_client_service() {
    let mut fixture = Fixture::new().await;
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(60);
    let (client, remote) = fixture.client().await;
    let path = fixture._root.path().join("credential-test.sock");
    let listener = crate::broker::linux::Listener::bind(&path, 1).unwrap();
    let uid = unsafe { libc::geteuid() };
    let rejected = Socket::connect(&path, uid).unwrap();
    // Exercise the real SO_PEERCRED rejection without changing process identity.
    let error = listener.accept(uid.wrapping_add(1)).err().unwrap();
    fixture.server.accepted(Err(error)).unwrap();
    assert!(rejected.receive(&mut [0; 4]).is_err());
    remote
        .send(
            &Message::Rejected {
                request: 3,
                reason: Rejection::Busy,
            },
            &[],
        )
        .await
        .unwrap();
    let (running, response) = tokio::join!(
        tokio::time::timeout(Duration::from_millis(200), fixture.server.run()),
        tokio::time::timeout(Duration::from_millis(150), remote.receive())
    );
    let retained = fixture.server.clients.contains_key(&client);
    remote.close();
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
    assert!(running.is_err());
    assert!(retained);
    assert!(matches!(
        response.unwrap().unwrap().message,
        Message::Rejected { request: 3, .. }
    ));
}
