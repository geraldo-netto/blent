//! T1619: each viewer requests recovery only when its own queue loses frames.
use super::*;

async fn playback() -> (ClientPlayback, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let viewer = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (server, _) = listener.accept().await.unwrap();
    (
        ClientPlayback {
            socket: server.into_split().1,
            last_sent_config: None,
            last_write: tokio::time::Instant::now(),
            capabilities: presence::channel().1,
            last_generation: None,
            wait_for_idr: false,
            idr_wanted: Default::default(),
            dropped: 0,
            scratch: VecDeque::new(),
        },
        viewer,
    )
}

fn packet(seq: u32) -> VideoPacket {
    VideoPacket {
        data: Bytes::from_static(b"frame"),
        is_idr: seq == 7,
        seq,
        codec_config: Some(Bytes::from_static(b"config")),
        generation: Arc::new(AtomicBool::new(true)),
    }
}

#[tokio::test]
async fn t1619_async_lag_requests_recovery_without_repeating_for_every_packet() {
    let (mut client, _viewer) = playback().await;
    let (tx, mut rx) = crate::video_queue::channel(4, client.idr_wanted.clone());
    let interval = blent_config::video::liveness::interval(2000).unwrap();
    tx.send(packet(0)).ok().unwrap();
    assert_eq!(
        client
            .next_packet(&mut rx, interval)
            .await
            .unwrap()
            .unwrap()
            .seq,
        0
    );
    assert!(!client.idr_wanted.load(Ordering::Acquire));
    for seq in 1..=8 {
        tx.send(packet(seq)).ok().unwrap();
    }
    assert_eq!(
        client
            .next_packet(&mut rx, interval)
            .await
            .unwrap()
            .unwrap()
            .seq,
        5
    );
    assert!(client.wait_for_idr);
    assert!(
        client.idr_wanted.swap(false, Ordering::AcqRel),
        "T1619 async lag did not request an IDR"
    );
    assert_eq!(
        client
            .next_packet(&mut rx, interval)
            .await
            .unwrap()
            .unwrap()
            .seq,
        6
    );
    assert!(!client.idr_wanted.load(Ordering::Acquire));
    drop(tx);
    assert_eq!(
        client
            .next_packet(&mut rx, interval)
            .await
            .unwrap()
            .unwrap()
            .seq,
        7
    );
    assert_eq!(
        client
            .next_packet(&mut rx, interval)
            .await
            .unwrap()
            .unwrap()
            .seq,
        8
    );
    assert!(client
        .next_packet(&mut rx, interval)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn t1619_drain_lag_requests_recovery_and_discards_old_dependencies() {
    let (mut client, _viewer) = playback().await;
    let (tx, mut rx) = crate::video_queue::channel(4, client.idr_wanted.clone());
    tx.send(packet(0)).ok().unwrap();
    let first = rx.try_recv().unwrap();
    for seq in 1..=8 {
        tx.send(packet(seq)).ok().unwrap();
    }
    let batch = client.drain_batch(&mut rx, first);
    assert_eq!(batch.iter().map(|p| p.seq).collect::<Vec<_>>(), [7, 8]);
    assert!(client.wait_for_idr);
    assert!(
        client.idr_wanted.swap(false, Ordering::AcqRel),
        "T1619 drain lag did not request an IDR"
    );
    drop(batch);
    tx.send(packet(9)).ok().unwrap();
    let first = rx.try_recv().unwrap();
    let batch = client.drain_batch(&mut rx, first);
    assert_eq!(batch.len(), 1);
    assert!(!client.idr_wanted.load(Ordering::Acquire));
    drop(batch);
    assert_eq!(tx.usage().0, 0);
}
