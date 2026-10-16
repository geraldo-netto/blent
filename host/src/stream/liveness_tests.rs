//! T1038: liveness owns no codec/frame/attachment authority.
use super::*;
use std::time::Duration;
use tokio::io::AsyncReadExt;

async fn pair() -> (ClientPlayback, TcpStream) {
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
            capabilities: negotiation::channel().1,
            last_generation: None,
            wait_for_idr: true,
            dropped: 0,
            scratch: VecDeque::new(),
        },
        viewer,
    )
}
async fn record(viewer: &TcpStream) -> Option<[u8; 6]> {
    let deadline = std::time::Instant::now() + Duration::from_millis(250);
    let mut bytes = [0; 6];
    let mut offset = 0;
    while offset < bytes.len() {
        if std::time::Instant::now() >= deadline {
            return None;
        }
        match viewer.try_read(&mut bytes[offset..]) {
            Ok(0) => return None,
            Ok(count) => offset += count,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::task::yield_now().await
            }
            Err(error) => panic!("T1038 read failed: {error}"),
        }
    }
    Some(bytes)
}
#[tokio::test(start_paused = true)]
async fn t1038_no_frames_for_longer_than_android_deadline_retain_original_viewer_and_fresh_resume()
{
    let (client, mut viewer) = pair().await;
    let (tx, rx) = crate::video_queue::channel(8, Default::default());
    let cache = CodecConfig::new(Some(Bytes::from_static(b"csd")));
    let (grant, capabilities) = negotiation::channel();
    grant.send_replace(true);
    let task = tokio::spawn(StreamServer::stream_packets(
        client.socket,
        rx,
        cache,
        tx.budget(),
        blent_config::video::liveness::interval(2000).unwrap(),
        capabilities,
    ));
    let mut initial = [0; 8];
    viewer.read_exact(&mut initial).await.unwrap();
    assert_eq!(&initial, b"\0\0\0\x04\0csd");
    let mut records = Vec::new();
    for _ in 0..6 {
        tokio::time::advance(Duration::from_secs(2)).await;
        records.push(record(&viewer).await);
        assert_eq!(tx.receiver_count(), 1);
        assert!(!task.is_finished());
    }
    tx.send(VideoPacket {
        seq: 71,
        is_idr: true,
        data: Bytes::from_static(b"fresh"),
        codec_config: Some(Bytes::from_static(b"new")),
        generation: Arc::new(AtomicBool::new(true)),
    })
    .ok()
    .unwrap();
    let mut resumed = [0; 22];
    viewer.read_exact(&mut resumed).await.unwrap();
    task.abort();
    let _ = task.await;
    assert_eq!(
        records,
        vec![Some([0, 0, 0, 2, 2, 1]); 6],
        "T1038 idle stream provided no independent liveness"
    );
    assert_eq!(&resumed[..8], b"\0\0\0\x04\0new");
    assert_eq!(&resumed[8..], b"\0\0\0\x0a\x01\0\0\0Gfresh");
}
#[tokio::test(start_paused = true)]
async fn t1038_ready_obsolete_packets_cannot_postpone_transport_liveness_or_revive_codec() {
    let (mut client, viewer) = pair().await;
    let (grant, capabilities) = negotiation::channel();
    grant.send_replace(true);
    client.capabilities = capabilities;
    let (tx, mut rx) = crate::video_queue::channel(8, Default::default());
    let mut records = Vec::new();
    for sequence in 0..6 {
        tokio::time::advance(Duration::from_secs(2)).await;
        tx.send(VideoPacket {
            seq: sequence,
            is_idr: true,
            data: Bytes::from_static(b"old"),
            codec_config: Some(Bytes::from_static(b"old")),
            generation: Arc::new(AtomicBool::new(false)),
        })
        .ok()
        .unwrap();
        let packet = client
            .next_packet(
                &mut rx,
                blent_config::video::liveness::interval(2000).unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        client.send_batch(VecDeque::from([packet])).await.unwrap();
        records.push(record(&viewer).await);
    }
    assert!(client.last_generation.is_none());
    assert!(client.last_sent_config.is_none());
    assert!(client.wait_for_idr);
    assert_eq!(
        records,
        vec![Some([0, 0, 0, 2, 2, 1]); 6],
        "T1038 stale producer traffic starved liveness"
    );
}
#[tokio::test(start_paused = true)]
async fn t1038_only_authenticated_current_client_gets_liveness_and_stop_releases_it() {
    let idr = Arc::new(AtomicBool::new(false));
    let server = Arc::new(StreamServer::new(
        StreamConfig {
            video_port: 0,
            token: Some("a".repeat(64)),
            ..Default::default()
        },
        CodecConfig::new(Some(Bytes::from_static(b"csd"))),
        idr.clone(),
    ));
    let listener = server.bind().await.unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = crate::video_queue::channel(8, Default::default());
    drop(rx);
    let owner = server.clone();
    let source = tx.clone();
    let task = tokio::spawn(async move { owner.run_with_listener(source, listener).await });
    let mut viewer = TcpStream::connect(address).await.unwrap();
    tokio::time::advance(Duration::from_secs(2)).await;
    let before = record(&viewer).await;
    assert_eq!(tx.receiver_count(), 0);
    assert!(!idr.load(Ordering::Acquire));
    viewer.write_all("a".repeat(64).as_bytes()).await.unwrap();
    viewer.read_exact(&mut [0; 8]).await.unwrap();
    tokio::time::advance(Duration::from_secs(12)).await;
    let legacy = record(&viewer).await;
    viewer.write_all(b"!BLENT1\n").await.unwrap();
    let after = record(&viewer).await;
    server.stop();
    task.await.unwrap().unwrap();
    assert_eq!(viewer.read(&mut [0]).await.unwrap(), 0);
    assert_eq!(tx.receiver_count(), 0);
    assert_eq!(before, None);
    assert_eq!(
        legacy, None,
        "T1038 legacy viewer received an unacknowledged packet type"
    );
    assert_eq!(
        after,
        Some([0, 0, 0, 2, 2, 1]),
        "T1038 authenticated idle connection lost liveness"
    );
}

#[tokio::test(start_paused = true)]
async fn t1038_actual_frame_writes_postpone_liveness_without_extra_packets() {
    let (mut client, mut viewer) = pair().await;
    let (grant, capabilities) = negotiation::channel();
    grant.send_replace(true);
    client.capabilities = capabilities;
    let (tx, mut rx) = crate::video_queue::channel(8, Default::default());
    let generation = Arc::new(AtomicBool::new(true));
    for sequence in 0..3 {
        tokio::time::advance(Duration::from_millis(1500)).await;
        tx.send(VideoPacket {
            seq: sequence,
            is_idr: sequence == 0,
            data: Bytes::from_static(b"ok"),
            codec_config: Some(Bytes::from_static(b"csd")),
            generation: generation.clone(),
        })
        .ok()
        .unwrap();
        let packet = client
            .next_packet(
                &mut rx,
                blent_config::video::liveness::interval(2000).unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        client.send_batch(VecDeque::from([packet])).await.unwrap();
        if sequence == 0 {
            viewer.read_exact(&mut [0; 8]).await.unwrap();
        }
        let mut frame = [0; 11];
        viewer.read_exact(&mut frame).await.unwrap();
        assert_eq!(frame, [0, 0, 0, 7, 1, 0, 0, 0, sequence as u8, b'o', b'k']);
    }
    assert_eq!(record(&viewer).await, None);
}
#[test]
fn t1038_configured_interval_rejects_invalid_values_and_honors_user_bounds() {
    const TEST: &str = "stream::liveness_tests::t1038_configured_interval_rejects_invalid_values_and_honors_user_bounds";
    if let Ok(expected) = std::env::var("BLENT_T1038_EXPECTED") {
        let result = liveness::configured_interval();
        match expected.parse::<u64>() {
            Ok(milliseconds) => assert_eq!(
                result.unwrap().duration(),
                Duration::from_millis(milliseconds)
            ),
            Err(_) => assert!(result.is_err()),
        }
        return;
    }
    for (value, expected) in [
        (None, "2000"),
        (Some("100"), "100"),
        (Some("5000"), "5000"),
        (Some("0"), "invalid"),
        (Some("99"), "invalid"),
        (Some("5001"), "invalid"),
        (Some("-1"), "invalid"),
        (Some(""), "invalid"),
        (Some("invalid"), "invalid"),
    ] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", TEST, "--nocapture"])
            .env("BLENT_T1038_EXPECTED", expected);
        match value {
            Some(value) => {
                command.env("BLENT_VIDEO_LIVENESS_MS", value);
            }
            None => {
                command.env_remove("BLENT_VIDEO_LIVENESS_MS");
            }
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test(start_paused = true)]
async fn t1038_unacknowledged_and_closed_original_grants_cannot_emit() {
    for closed in [false, true] {
        let (mut client, viewer) = pair().await;
        let (grant, capabilities) = negotiation::channel();
        if closed {
            grant.send_replace(true);
        }
        client.capabilities = capabilities;
        if closed {
            drop(grant);
        }
        client.send_liveness().await.unwrap();
        assert_eq!(
            record(&viewer).await,
            None,
            "T1038 denied original grant emitted"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn t1038_late_ack_wakes_idle_playback_without_producer_packet() {
    let (mut client, viewer) = pair().await;
    let (grant, capabilities) = negotiation::channel();
    client.capabilities = capabilities;
    let (tx, mut rx) = crate::video_queue::channel(8, Default::default());
    let task = tokio::spawn(async move {
        client
            .next_packet(
                &mut rx,
                blent_config::video::liveness::interval(2000).unwrap(),
            )
            .await
    });
    tokio::time::advance(Duration::from_secs(12)).await;
    assert_eq!(
        record(&viewer).await,
        None,
        "T1038 unacknowledged idle socket emitted"
    );
    grant.send_replace(true);
    assert_eq!(record(&viewer).await, Some([0, 0, 0, 2, 2, 1]));
    drop(tx);
    assert!(task.await.unwrap().unwrap().is_none());
}

#[tokio::test(start_paused = true)]
async fn t1038_failed_liveness_write_does_not_renew_deadline() {
    let (mut client, _viewer) = pair().await;
    let (grant, capabilities) = negotiation::channel();
    grant.send_replace(true);
    client.capabilities = capabilities;
    client.socket.shutdown().await.unwrap();
    let original = client.last_write;
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(client.send_liveness().await.is_err());
    assert_eq!(client.last_write, original);
}

#[tokio::test]
async fn t1038_lagged_legacy_viewer_still_waits_for_fresh_idr() {
    let (mut client, viewer) = pair().await;
    client.wait_for_idr = false;
    let (tx, mut rx) = crate::video_queue::channel(1, Default::default());
    for sequence in 0..32 {
        tx.send(VideoPacket {
            seq: sequence,
            is_idr: true,
            data: Bytes::from_static(b"ok"),
            codec_config: None,
            generation: Arc::new(AtomicBool::new(true)),
        })
        .ok()
        .unwrap();
    }
    assert!(client
        .next_packet(
            &mut rx,
            blent_config::video::liveness::interval(2000).unwrap()
        )
        .await
        .unwrap()
        .is_some());
    assert!(client.wait_for_idr);
    assert_eq!(record(&viewer).await, None);
}

#[tokio::test(start_paused = true)]
async fn t1038_stalled_liveness_write_retains_whole_record_deadline() {
    let (mut writer, _blocked_peer) = tokio::io::duplex(1);
    let start = tokio::time::Instant::now();
    assert!(StreamServer::write_packet(&mut writer, 2, &[1])
        .await
        .is_err());
    assert_eq!(tokio::time::Instant::now() - start, WRITE_TIMEOUT);
}
