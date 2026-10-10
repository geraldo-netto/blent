use super::*;
use tokio::io::{duplex, DuplexStream};

async fn tablet(mut stream: DuplexStream, capacity: u32) -> u32 {
    let mut proposal = [0; window::BYTES];
    stream.read_exact(&mut proposal).await.unwrap();
    let selected = window::select(window::decode(&proposal, true).unwrap(), capacity).unwrap();
    // Fragmented stream is independent of native USB transfer boundaries.
    for byte in window::encode(false, selected).unwrap() {
        stream.write_all(&[byte]).await.unwrap();
    }
    stream.write_all(b"next").await.unwrap();
    selected
}

#[tokio::test]
async fn t875_different_local_limits_agree_and_preserve_next_frame() {
    for capacity in [4096, 65536, 1048576] {
        let (mut host, remote) = duplex(64);
        let (limits, selected) = tokio::join!(
            negotiate(&mut host, Limits::default()),
            tablet(remote, capacity)
        );
        let limits = limits.unwrap();
        assert_eq!(limits.channel_bytes, selected);
        assert_eq!(limits.channel_bytes, capacity.min(262144));
        assert_eq!(limits.queued_messages, 64);
        assert_eq!(limits.quantum, 65536);
        let mut tail = [0; 4];
        host.read_exact(&mut tail).await.unwrap();
        assert_eq!(&tail, b"next");
    }
}

async fn reject_reply(bytes: &[u8]) {
    let (mut host, mut remote) = duplex(64);
    let (_, result) = tokio::join!(
        async {
            let mut proposal = [0; window::BYTES];
            remote.read_exact(&mut proposal).await.unwrap();
            remote.write_all(bytes).await.unwrap();
            remote.shutdown().await.unwrap();
        },
        negotiate(&mut host, Limits::default())
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn t875_bad_reply_truncation_role_and_expansion_rejected() {
    let valid = window::encode(false, 4096).unwrap();
    for length in 0..window::BYTES {
        reject_reply(&valid[..length]).await;
    }
    reject_reply(&window::encode(true, 4096).unwrap()).await;
    reject_reply(&window::encode(false, 1048576).unwrap()).await;
    let mut bad = valid;
    bad[8..].copy_from_slice(&0u32.to_be_bytes());
    reject_reply(&bad).await;
    bad[4] = 2;
    reject_reply(&bad).await;
}

#[tokio::test(start_paused = true)]
async fn t875_absolute_deadline_includes_partial_writes_and_reads() {
    let (mut host, _remote) = duplex(1);
    let start = tokio::time::Instant::now();
    assert!(negotiate(&mut host, Limits::default()).await.is_err());
    assert_eq!(start.elapsed(), Duration::from_secs(5));

    let (mut host, mut remote) = duplex(64);
    let start = tokio::time::Instant::now();
    let (_, result) = tokio::join!(
        async {
            let mut proposal = [0; window::BYTES];
            remote.read_exact(&mut proposal).await.unwrap();
            for byte in window::encode(false, 4096).unwrap().iter().take(4) {
                tokio::time::sleep(Duration::from_secs(1)).await;
                remote.write_all(&[*byte]).await.unwrap();
            }
        },
        negotiate(&mut host, Limits::default())
    );
    assert!(result.is_err());
    assert_eq!(start.elapsed(), Duration::from_secs(5));
}

#[tokio::test]
async fn t875_invalid_local_settings_send_no_bytes_and_closed_stream_fails() {
    for capacity in [0, 4095, 1048577, u32::MAX] {
        let (mut host, mut remote) = duplex(64);
        assert!(negotiate(
            &mut host,
            Limits {
                channel_bytes: capacity,
                ..Limits::default()
            }
        )
        .await
        .is_err());
        drop(host);
        assert_eq!(remote.read(&mut [0]).await.unwrap(), 0);
    }
    for io_timeout in [Duration::ZERO, Duration::from_secs(61)] {
        let (mut host, _remote) = duplex(64);
        assert!(negotiate(
            &mut host,
            Limits {
                io_timeout,
                ..Limits::default()
            }
        )
        .await
        .is_err());
    }
    let (mut host, remote) = duplex(64);
    drop(remote);
    assert!(negotiate(&mut host, Limits::default()).await.is_err());
}
