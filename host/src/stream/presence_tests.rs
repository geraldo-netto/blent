//! T1567/T1039: current liveness is unconditional; presence stays socket-owned.
use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::{timeout, Duration},
};

#[tokio::test]
async fn t1039_original_connection_closure_revokes_only_its_presence() {
    let (peer, mut reader) = tokio::io::duplex(128);
    let (publish, mut original) = presence::channel();
    let (_other, replacement) = presence::channel();
    assert!(original.supported());
    assert!(replacement.supported());
    let reader = tokio::spawn(async move { presence::watch(&mut reader, publish).await });
    drop(peer);
    let _ = reader.await.unwrap();
    assert!(!original.supported());
    assert!(replacement.supported());
    assert!(timeout(Duration::from_millis(10), original.changed())
        .await
        .is_err());
}
async fn rejected(record: &[u8]) -> bool {
    let (mut peer, mut reader) = tokio::io::duplex(256);
    let (publish, observed) = presence::channel();
    peer.write_all(record).await.unwrap();
    peer.shutdown().await.unwrap();
    let result = presence::watch(&mut reader, publish).await;
    assert!(!observed.supported());
    result.is_err()
}
#[tokio::test]
async fn t1039_obsolete_ack_and_optional_token_records_always_reject() {
    // Retains the strict bounded length/byte mutation domain of the removed
    // common fixed-ACK parser; no obsolete record may prolong socket ownership.
    for length in 0..=32 {
        assert!(rejected(&vec![0; length]).await);
    }
    for length in 0..=8 {
        assert!(rejected(&b"!BLENT1\n"[..length]).await);
    }
    for offset in 0..8 {
        for value in 0..=255 {
            let mut bytes = *b"!BLENT1\n";
            bytes[offset] = value;
            assert!(rejected(&bytes).await);
        }
    }
    let token = "a".repeat(64);
    assert!(rejected(token.as_bytes()).await);
    assert!(rejected(format!("{token}!BLENT1\n").as_bytes()).await);
    assert!(rejected(format!("{}x!BLENT1\n", "a".repeat(63)).as_bytes()).await);
}
#[tokio::test(start_paused = true)]
async fn t1039_first_obsolete_ack_byte_rejects_without_waiting_for_a_record() {
    let (mut peer, mut reader) = tokio::io::duplex(128);
    let (publish, observed) = presence::channel();
    peer.write_all(b"!").await.unwrap();
    let mut work = Box::pin(presence::watch(&mut reader, publish));
    assert!(matches!(
        futures_util::poll!(&mut work),
        std::task::Poll::Ready(Err(_))
    ));
    assert!(!observed.supported());
}
#[tokio::test(start_paused = true)]
async fn t1039_current_silent_socket_retains_presence_without_periodic_polling() {
    let (_peer, mut reader) = tokio::io::duplex(128);
    let (publish, observed) = presence::channel();
    let mut work = Box::pin(presence::watch(&mut reader, publish));
    assert!(futures_util::poll!(&mut work).is_pending());
    tokio::time::advance(Duration::from_secs(60)).await;
    assert!(futures_util::poll!(&mut work).is_pending());
    assert!(observed.supported());
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
