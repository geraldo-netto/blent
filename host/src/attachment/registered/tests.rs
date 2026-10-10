use super::*;
use crate::media::EncoderSettings;
use tokio::sync::watch;

fn settings() -> EncoderSettings {
    EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: true,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    }
}
fn peer(value: u64) -> PeerId {
    PeerId::parse(format!("{value:064x}")).unwrap()
}
fn fixture(token: Option<String>) -> (Attachment, watch::Receiver<EncoderSettings>) {
    let (sender, receiver) = watch::channel(settings());
    (Attachment::with_token(sender, token), receiver)
}
#[test]
fn t916_concurrent_handoffs_capture_their_own_atomic_credential_and_identity() {
    let (attachment, _) = fixture(None);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    std::thread::scope(|scope| {
        let mut threads = Vec::new();
        for owner in 0..8 {
            let attachment = attachment.clone();
            let barrier = barrier.clone();
            threads.push(scope.spawn(move || {
                barrier.wait();
                for iteration in 0..256 {
                    let id = owner * 256 + iteration + 1;
                    let peer = peer(id);
                    let credential = Credential::new([id as u8; 32]);
                    let expected = token(&credential);
                    let route = attachment
                        .register_route(peer.clone(), credential, Link::Usb)
                        .unwrap();
                    assert_eq!(
                        route.lease.token(None).unwrap(),
                        Some(expected.as_str()),
                        "T916 foreign handoff credential"
                    );
                    assert_eq!(route.lease.identity.as_ref().unwrap().peer(), Some(&peer));
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    });
}

#[test]
fn t916_required_native_credentials_override_legacy_tokenless_and_fallback() {
    for initial in [None, Some("legacy-secret".into())] {
        let (attachment, settings) = fixture(initial.clone());
        let presence = attachment.subscribe();
        let legacy = attachment.lease();
        let route = attachment
            .register_route(peer(1), Credential::new([0xff; 32]), Link::Usb)
            .unwrap();
        assert!(!legacy.apply(|| panic!("stale legacy authority")));
        assert_eq!(attachment.token().unwrap(), Some("ff".repeat(32)));
        assert_eq!(
            route.lease.token(Some("wrong-fallback")).unwrap(),
            Some("ff".repeat(32).as_str())
        );
        assert_eq!(
            route.lease.profile_identity(),
            Some((format!("certificate:{}", peer(1).as_str()), "usb"))
        );
        assert!(!*presence.borrow());
        assert!(!settings.borrow().geometry_ready);
        assert!(route.admitted());
        assert!(*presence.borrow());
        attachment.begin(Some("device:legacy".into()));
        let token = attachment.token().unwrap();
        assert_eq!(token.is_some(), initial.is_some());
        assert_ne!(token, Some("ff".repeat(32)));
        assert!(!route.admitted());
        route.close();
        assert_eq!(attachment.token().unwrap(), token);
    }
}
#[test]
fn t916_same_peer_renewal_preserves_geometry_and_presence_but_retires_old_sockets() {
    let (attachment, settings) = fixture(None);
    let presence = attachment.subscribe();
    let first = attachment
        .register_route(peer(1), Credential::new([1; 32]), Link::Usb)
        .unwrap();
    assert!(first.admitted());
    attachment
        .0
        .settings
        .send_modify(|settings| settings.geometry_ready = true);
    let socket = attachment.lease();
    first.close();
    assert!(!socket.apply(|| panic!("revoked socket")));
    assert!(!first.admitted());
    assert!(attachment.token().is_err());
    assert!(*presence.borrow());
    assert!(settings.borrow().geometry_ready);
    let second = attachment
        .register_route(peer(1), Credential::new([2; 32]), Link::Usb)
        .unwrap();
    assert!(*presence.borrow());
    assert!(settings.borrow().geometry_ready);
    assert!(second.admitted());
    drop(first);
    assert_eq!(attachment.token().unwrap(), Some("02".repeat(32)));
    assert!(second.admitted());
    let third = attachment
        .register_route(peer(2), Credential::new([3; 32]), Link::Network)
        .unwrap();
    assert!(!*presence.borrow());
    assert!(!settings.borrow().geometry_ready);
    assert!(!second.admitted());
    assert_eq!(third.lease.transport(), Some("network"));
    assert!(third.admitted());
}
#[tokio::test]
async fn t916_drop_revokes_credential_and_wakes_current_socket_retirement() {
    let (attachment, _) = fixture(None);
    let route = attachment
        .register_route(peer(1), Credential::new([7; 32]), Link::Usb)
        .unwrap();
    let mut retired = route.lease.retirement();
    let socket = attachment.lease();
    drop(route);
    tokio::time::timeout(std::time::Duration::from_secs(1), retired.retired())
        .await
        .unwrap();
    assert!(attachment.token().is_err());
    assert!(attachment.lease().token(Some("fallback")).is_err());
    assert!(!socket.apply(|| panic!("dropped route remained authorized")));
}
#[tokio::test]
async fn t916_generation_exhaustion_never_wraps_or_restores_retired_authority() {
    let (attachment, _) = fixture(None);
    attachment.0.state.lock().unwrap().generation = u64::MAX - 1;
    let route = attachment
        .register_route(peer(1), Credential::new([8; 32]), Link::Usb)
        .unwrap();
    assert_eq!(route.lease.generation, u64::MAX);
    assert!(route.admitted());
    let mut retirement = route.lease.retirement();
    route.close();
    assert!(!route.admitted());
    assert_eq!(attachment.0.state.lock().unwrap().generation, u64::MAX);
    assert!(!*attachment.subscribe().borrow());
    assert!(attachment.token().is_err());
    assert!(attachment
        .register_route(peer(1), Credential::new([9; 32]), Link::Usb)
        .is_err());
    attachment.begin(Some("device:late".into()));
    assert!(attachment.token().is_err());
    tokio::time::timeout(std::time::Duration::from_secs(1), retirement.retired())
        .await
        .unwrap();
}
#[test]
fn t916_closed_session_and_stale_native_close_cannot_mutate_replacement() {
    let (attachment, _) = fixture(None);
    let old = attachment
        .register_route(peer(1), Credential::new([1; 32]), Link::Usb)
        .unwrap();
    let current = attachment
        .register_route(peer(1), Credential::new([2; 32]), Link::Usb)
        .unwrap();
    old.close();
    assert_eq!(attachment.token().unwrap(), Some("02".repeat(32)));
    assert!(current.admitted());
    attachment.retire();
    assert!(attachment
        .register_route(peer(1), Credential::new([3; 32]), Link::Usb)
        .is_err());
    assert!(!current.admitted());
    current.close();
    assert!(attachment.token().is_err());
}
#[test]
fn t916_unmanaged_legacy_fallback_is_restored_without_caching_native_secrets() {
    let (settings, _) = watch::channel(settings());
    let attachment = Attachment::new(settings);
    let route = attachment
        .register_route(peer(1), Credential::new([1; 32]), Link::Usb)
        .unwrap();
    attachment.begin(Some("device:standalone".into()));
    assert_eq!(
        attachment.lease().token(Some("original-fallback")).unwrap(),
        Some("original-fallback")
    );
    assert!(!route.admitted());
    assert!(matches!(
        Authentication::Unmanaged.renewable(),
        Authentication::Unmanaged
    ));
    assert!(matches!(
        Authentication::Unavailable.renewable(),
        Authentication::Unavailable
    ));
}

#[test]
fn t916_absence_does_not_restore_tokenless_authentication_while_old_route_retires() {
    for legacy in [None, Some("old-legacy".into())] {
        let (attachment, _) = fixture(legacy.clone());
        let _presence = attachment.subscribe();
        let route = attachment
            .register_route(peer(1), Credential::new([5; 32]), Link::Usb)
            .unwrap();
        assert!(route.admitted());
        attachment.send(false).unwrap();
        assert!(
            attachment.token().is_err(),
            "T916 absence must retain credential revocation"
        );
        assert!(attachment.lease().token(Some("fallback")).is_err());
        assert!(!route.admitted());
        attachment.begin_route(Some(Identity::authenticated(peer(1))), Some(Link::Usb));
        assert!(
            attachment.token().is_err(),
            "T916 descriptive identity is not registered authority"
        );
        attachment.begin(Some("device:explicit-legacy-fallback".into()));
        assert_eq!(attachment.token().unwrap().is_some(), legacy.is_some());
    }
}

#[test]
fn t916_native_detach_and_legacy_fallback_fail_closed_at_generation_exhaustion() {
    for fallback in [None, Some("device:legacy".into())] {
        let (attachment, _) = fixture(None);
        attachment.0.state.lock().unwrap().generation = u64::MAX - 1;
        let route = attachment
            .register_route(peer(1), Credential::new([1; 32]), Link::Usb)
            .unwrap();
        attachment.begin(fallback);
        assert_eq!(
            attachment.0.state.lock().unwrap().generation,
            u64::MAX,
            "T916 native handoff wrapped"
        );
        assert!(attachment.token().is_err());
        assert!(!route.admitted());
        assert!(attachment
            .register_route(peer(1), Credential::new([2; 32]), Link::Usb)
            .is_err());
    }
}

#[tokio::test]
async fn t916_route_revoke_cancels_pending_socket_io_before_replacement_admission() {
    let (attachment, _) = fixture(None);
    let route = attachment
        .register_route(peer(1), Credential::new([1; 32]), Link::Usb)
        .unwrap();
    let socket = attachment.lease();
    let (entered, entry) = tokio::sync::oneshot::channel();
    let io = socket.run(async {
        entered.send(()).unwrap();
        std::future::pending().await
    });
    tokio::pin!(io);
    tokio::select! { result=&mut io=>panic!("T916 pending socket returned {result:?}"), _=entry=>() }
    route.close();
    tokio::time::timeout(std::time::Duration::from_secs(1), &mut io)
        .await
        .unwrap()
        .unwrap();
    socket
        .run(async { panic!("T916 retired socket was polled") })
        .await
        .unwrap();
    let next = attachment
        .register_route(peer(1), Credential::new([2; 32]), Link::Usb)
        .unwrap();
    assert!(next.admitted());
    let live = attachment.lease();
    assert!(live
        .run(async { anyhow::bail!("T916 native I/O failure") })
        .await
        .is_err());
    live.run(async { Ok(()) }).await.unwrap();
}
