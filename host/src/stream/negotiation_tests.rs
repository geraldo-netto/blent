//! T1039: explicit grants belong to one authenticated original video socket.
use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
    time::{timeout, Duration},
};

#[tokio::test]
async fn t1039_complete_ack_grants_only_original_connection_and_closure_revokes_view() {
    let (mut peer, mut reader) = tokio::io::duplex(128);
    let (publish, mut original) = negotiation::channel();
    let (_other, replacement) = negotiation::channel();
    let reader = tokio::spawn(async move { negotiation::watch(&mut reader, false, publish).await });
    peer.write_all(b"!BLENT1\n").await.unwrap();
    let changed = timeout(Duration::from_millis(50), original.changed()).await;
    let granted = original.supported();
    assert!(!replacement.supported());
    drop(peer);
    let _ = reader.await.unwrap();
    assert!(!original.supported());
    assert!(timeout(Duration::from_millis(10), original.changed())
        .await
        .is_err());
    assert!(
        changed.is_ok() && granted,
        "T1039 exact acknowledgement did not grant original connection"
    );
}
async fn parsed(record: &[u8], optional: bool) -> bool {
    let (mut peer, mut reader) = tokio::io::duplex(256);
    let (publish, observed) = watch::channel(false);
    peer.write_all(record).await.unwrap();
    peer.shutdown().await.unwrap();
    let _ = negotiation::watch(&mut reader, optional, publish).await;
    let accepted = *observed.borrow();
    accepted
}
#[tokio::test]
async fn t1039_fixed_ack_and_optional_token_have_strict_bounded_records() {
    for optional in [false, true] {
        assert!(parsed(b"!BLENT1\n", optional).await);
        for length in 0..8 {
            assert!(!parsed(&b"!BLENT1\n"[..length], optional).await);
        }
        for offset in 0..8 {
            for value in 0..=255 {
                let mut bytes = *b"!BLENT1\n";
                bytes[offset] = value;
                assert_eq!(
                    parsed(&bytes, optional).await,
                    value == b"!BLENT1\n"[offset]
                );
            }
        }
    }
    let token = "a".repeat(64);
    assert!(!parsed(token.as_bytes(), true).await);
    let acknowledged = format!("{token}!BLENT1\n");
    assert!(parsed(acknowledged.as_bytes(), true).await);
    assert!(!parsed(acknowledged.as_bytes(), false).await);
    assert!(!parsed(format!("{}x!BLENT1\n", "a".repeat(63)).as_bytes(), true).await);
}
#[tokio::test(start_paused = true)]
async fn t1039_first_ack_byte_keeps_original_three_second_completion_deadline() {
    let (mut peer, mut reader) = tokio::io::duplex(128);
    let (publish, observed) = watch::channel(false);
    peer.write_all(b"!").await.unwrap();
    let mut work = Box::pin(negotiation::watch(&mut reader, false, publish));
    assert!(futures_util::poll!(&mut work).is_pending());
    tokio::time::advance(Duration::from_secs(3)).await;
    assert!(work.await.is_err());
    assert!(!*observed.borrow());
}
#[tokio::test(start_paused = true)]
async fn t1039_legacy_silence_does_not_grant_capabilities_or_add_periodic_polling() {
    let (_peer, mut reader) = tokio::io::duplex(128);
    let (publish, observed) = watch::channel(false);
    let mut work = Box::pin(negotiation::watch(&mut reader, true, publish));
    assert!(futures_util::poll!(&mut work).is_pending());
    tokio::time::advance(Duration::from_secs(60)).await;
    assert!(futures_util::poll!(&mut work).is_pending());
    assert!(!*observed.borrow());
}
#[tokio::test]
async fn t1039_authentication_failure_never_processes_feature_ack_or_admits_viewer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let (video, receiver) = crate::video_queue::channel(8, Default::default());
    drop(receiver);
    let watching = video.clone();
    let idr = Arc::new(AtomicBool::new(false));
    let requested = idr.clone();
    let server = tokio::spawn(StreamServer::handle_client(
        socket,
        video,
        Default::default(),
        Some("a".repeat(64)),
        idr,
    ));
    client
        .write_all(format!("{}!BLENT1\n", "b".repeat(64)).as_bytes())
        .await
        .unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(watching.receiver_count(), 0);
    assert!(!requested.load(Ordering::Acquire));
    let closed = client.read(&mut [0]).await;
    assert!(closed.is_err() || closed.unwrap() == 0);
}
