//! T1633: real accept exhaustion must back off without delaying stop or recovery.
use std::{future::Future, sync::Arc, time::Duration};
use tokio::net::{TcpListener, TcpStream};

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

async fn exhaust<F: Future>(
    future: &mut std::pin::Pin<Box<F>>,
    address: std::net::SocketAddr,
) -> TcpStream {
    let probe = Arc::new(crate::poll_probe::Probe::default());
    assert!(probe.poll(future.as_mut()).is_pending());
    let peer = TcpStream::connect(address).await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    probe.take();
    let limit = FileLimit::exhausted();
    for _ in 0..4 {
        assert!(probe.poll(future.as_mut()).is_pending());
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    drop(limit);
    let failures = crate::test_logging::text()
        .to_lowercase()
        .matches("accept failed:")
        .count();
    assert_eq!(
        failures, 1,
        "T1633 repeated native accept attempts before the 100ms retry boundary"
    );
    peer
}

#[tokio::test]
async fn t1633_stream_exhaustion_backs_off_and_stop_remains_immediate() {
    if crate::test_logging::isolated(
        "accept_failure_tests::t1633_stream_exhaustion_backs_off_and_stop_remains_immediate",
    ) {
        return;
    }
    let server = crate::stream::StreamServer::new(
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (video, _) = crate::video_queue::channel(8, Default::default());
    let mut future = Box::pin(server.run_with_listener(video, listener));
    let _peer = exhaust(&mut future, address).await;
    server.stop();
    tokio::time::timeout(Duration::from_millis(50), future)
        .await
        .expect("T1633 stop waited for retry deadline")
        .unwrap();
}

#[tokio::test]
async fn t1633_input_exhaustion_backs_off_and_cancellation_retires_owner() {
    if crate::test_logging::isolated(
        "accept_failure_tests::t1633_input_exhaustion_backs_off_and_cancellation_retires_owner",
    ) {
        return;
    }
    let (mode, _) = tokio::sync::watch::channel(false);
    let (_card, card) = tokio::sync::watch::channel(None);
    let (_tablet, tablet) = tokio::sync::watch::channel(false);
    let server = crate::input::InputServer::new(
        crate::input::InputConfig {
            touch: false,
            pen: false,
            pointer: false,
            ..Default::default()
        },
        None,
        mode,
        Default::default(),
        Default::default(),
        card,
        tablet,
    )
    .unwrap();
    let original = server.native_completion().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut future = Box::pin(server.run_with_listener(listener));
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut peer = exhaust(&mut future, address).await;
    peer.shutdown().await.unwrap();
    let mut byte = [0];
    tokio::time::timeout(Duration::from_millis(500), async {
        tokio::select! {
            result = &mut future => panic!("server unexpectedly ended: {result:?}"),
            result = peer.read(&mut byte) => assert_eq!(result.unwrap(), 0),
        }
    })
    .await
    .expect("T1633 input listener did not recover after descriptors returned");
    drop(future);
    drop(server);
    tokio::time::timeout(Duration::from_secs(3), async {
        while !original.is_complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1633_stream_recovers_after_descriptor_capacity_returns() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    if crate::test_logging::isolated(
        "accept_failure_tests::t1633_stream_recovers_after_descriptor_capacity_returns",
    ) {
        return;
    }
    let server = crate::stream::StreamServer::new(
        crate::stream::StreamConfig {
            video_port: 0,
            token: Some("a".repeat(64)),
        },
        Default::default(),
        Default::default(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (video, _) = crate::video_queue::channel(8, Default::default());
    let mut future = Box::pin(server.run_with_listener(video, listener));
    let mut peer = exhaust(&mut future, address).await;
    peer.write_all("b".repeat(64).as_bytes()).await.unwrap();
    let mut byte = [0];
    tokio::time::timeout(Duration::from_millis(500), async {
        tokio::select! {
            result = &mut future => panic!("server unexpectedly ended: {result:?}"),
            result = peer.read(&mut byte) => assert_eq!(result.unwrap(), 0),
        }
    })
    .await
    .expect("T1633 listener did not recover after descriptors returned");
    server.stop();
    future.await.unwrap();
}
