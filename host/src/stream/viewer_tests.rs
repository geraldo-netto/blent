//! T1041: capture observes original grants, never a copied capability flag.
use super::*;
use crate::capture_contract::{PacketOutput, ViewerActivity};
use tokio::{
    io::AsyncReadExt,
    time::{timeout, Duration},
};

async fn ready(viewers: &dyn ViewerActivity) {
    timeout(Duration::from_millis(100), async {
        loop {
            let changed = viewers.changed();
            if viewers.liveness_ready() {
                return;
            }
            changed.await;
        }
    })
    .await
    .expect("T1041 negotiated viewer capability did not reach capture");
}

#[tokio::test]
async fn t1041_capture_observes_original_grant_and_denies_unmanaged_or_closed_sources() {
    let (video, raw) = crate::video_queue::channel(8, Default::default());
    let viewers = video.viewers();
    assert_eq!(viewers.epoch(), 0);
    let (grant, capability) = negotiation::channel();
    let original = video.subscribe_negotiated(Arc::new(capability));
    assert_eq!(viewers.epoch(), 1);
    assert_eq!(video.viewer_epoch().load(Ordering::Acquire), 1);
    grant.send_replace(true);
    assert!(
        !viewers.liveness_ready(),
        "T1041 unmanaged consumer was ignored"
    );
    drop(raw);
    ready(viewers.as_ref()).await;
    drop(grant);
    assert!(
        !viewers.liveness_ready(),
        "T1041 closed grant remained eligible"
    );
    drop(original);
    assert!(!viewers.liveness_ready());
}

#[tokio::test]
async fn t1041_authenticated_socket_ack_reaches_capture_and_peer_loss_revokes_it() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (socket, _) = listener.accept().await.unwrap();
    let (video, raw) = crate::video_queue::channel(8, Default::default());
    drop(raw);
    let viewers = video.viewers();
    let owner = tokio::spawn(StreamServer::handle_client(
        socket,
        video.clone(),
        CodecConfig::new(Some(Bytes::from_static(b"csd"))),
        Some("a".repeat(64)),
        Default::default(),
    ));
    peer.write_all("a".repeat(64).as_bytes()).await.unwrap();
    peer.read_exact(&mut [0; 8]).await.unwrap();
    assert!(!viewers.liveness_ready());
    peer.write_all(b"!BLENT1\n").await.unwrap();
    ready(viewers.as_ref()).await;
    drop(peer);
    let _ = owner.await.unwrap();
    assert_eq!(video.receiver_count(), 0);
    assert!(!viewers.liveness_ready());
}

#[tokio::test]
async fn t1041_armed_original_grant_notice_cannot_lose_closure_before_first_poll() {
    let (grant, capability) = negotiation::channel();
    grant.send_replace(true);
    let original: &dyn crate::capture_contract::LivenessCapability = &capability;
    let changed = original.changed();
    assert!(original.supported());
    drop(grant);
    timeout(Duration::from_millis(20), changed)
        .await
        .expect("T1041 closure between arm and first poll was lost");
}

#[tokio::test]
async fn t1041_mixed_original_viewers_wake_on_ack_and_legacy_departure() {
    let (video, initial) = crate::video_queue::channel(8, Default::default());
    drop(initial);
    let viewers = video.viewers();
    let (first, capability) = negotiation::channel();
    let _first = video.subscribe_negotiated(Arc::new(capability));
    first.send_replace(true);
    assert!(viewers.liveness_ready());
    let added = viewers.changed();
    let (second, capability) = negotiation::channel();
    let second_reader = video.subscribe_negotiated(Arc::new(capability));
    assert!(!viewers.liveness_ready());
    timeout(Duration::from_millis(100), added).await.unwrap();
    let acknowledged = viewers.changed();
    second.send_replace(true);
    timeout(Duration::from_millis(100), acknowledged)
        .await
        .unwrap();
    assert!(viewers.liveness_ready());
    let legacy = video.subscribe();
    let departed = viewers.changed();
    assert!(!viewers.liveness_ready());
    drop(legacy);
    timeout(Duration::from_millis(100), departed).await.unwrap();
    assert!(viewers.liveness_ready());
    let lost = viewers.changed();
    drop(second);
    timeout(Duration::from_millis(100), lost).await.unwrap();
    assert!(!viewers.liveness_ready());
    drop(second_reader);
    assert!(viewers.liveness_ready());
}

#[tokio::test]
async fn t1041_late_original_notices_never_grant_replacement_membership() {
    let (video, initial) = crate::video_queue::channel(8, Default::default());
    drop(initial);
    let viewers = video.viewers();
    for _ in 0..1000 {
        let (old, capability) = negotiation::channel();
        let old_reader = video.subscribe_negotiated(Arc::new(capability));
        old.send_replace(true);
        let retained_notice = viewers.changed();
        drop(old_reader);
        let (new, capability) = negotiation::channel();
        let replacement = video.subscribe_negotiated(Arc::new(capability));
        old.send_replace(true);
        timeout(Duration::from_millis(100), retained_notice)
            .await
            .unwrap();
        assert!(!viewers.liveness_ready());
        new.send_replace(true);
        assert!(viewers.liveness_ready());
        drop(replacement);
        assert_eq!(video.receiver_count(), 0);
        assert!(!viewers.liveness_ready());
    }
}

#[tokio::test]
async fn t1041_read_only_wait_is_idle_and_closed_sources_do_not_spin() {
    let (video, initial) = crate::video_queue::channel(8, Default::default());
    drop(initial);
    let viewers = video.viewers();
    assert!(timeout(Duration::from_millis(10), viewers.changed())
        .await
        .is_err());
    let (grant, capability) = negotiation::channel();
    grant.send_replace(true);
    let original = video.subscribe_negotiated(Arc::new(capability));
    drop(grant);
    assert!(!viewers.liveness_ready());
    assert!(timeout(Duration::from_millis(10), viewers.changed())
        .await
        .is_err());
    drop(original);
    let unsupported = std::sync::atomic::AtomicU64::new(0);
    assert!(!unsupported.liveness_ready());
    assert!(timeout(Duration::from_millis(10), unsupported.changed())
        .await
        .is_err());
}
