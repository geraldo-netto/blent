//! T856: ADB/native adapters share portable attachment metadata, not identity parsing.
use blent::{attachment::Attachment, media::EncoderSettings};
use blent_config::connection::{
    pairing::PeerId,
    routes::{Identity, Link},
};
fn attachment() -> (Attachment, tokio::sync::watch::Receiver<EncoderSettings>) {
    let settings = EncoderSettings {
        encoder: "libx264".into(),
        fps: 30,
        bitrate: 20000,
        width: 640,
        height: 480,
        quality: 18,
        width_mm: 310,
        height_mm: 194,
        stream_scale: 1,
        geometry_ready: false,
        decoders: None,
        decoder_epoch: 0,
        selection: None,
    };
    let (tx, rx) = tokio::sync::watch::channel(settings);
    (Attachment::with_token(tx, Some("a".repeat(64))), rx)
}
#[test]
fn t856_native_identity_needs_no_adb_prefix_and_retires_previous_lease() {
    let (attachment, _) = attachment();
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    let identity = Identity::authenticated(peer.clone());
    attachment.begin_route(Some(identity.clone()), Some(Link::Usb));
    let first = attachment.lease();
    assert_eq!(
        first.profile_identity(),
        Some((format!("certificate:{}", peer.as_str()), "usb"))
    );
    assert!(first.apply(|| {}));
    let token = attachment.token().unwrap();
    attachment.begin_route(Some(identity), Some(Link::Network));
    assert!(!first.apply(|| panic!("stale route revived")));
    assert_eq!(attachment.token().unwrap(), token);
    assert_eq!(attachment.lease().transport(), Some("network"));
    attachment.begin_route(
        Some(Identity::authenticated(
            PeerId::parse("cd".repeat(32)).unwrap(),
        )),
        Some(Link::Usb),
    );
    assert_ne!(attachment.token().unwrap(), token);
    attachment.retire();
    attachment.begin_route(Some(Identity::authenticated(peer)), Some(Link::Usb));
    assert!(attachment.token().is_err());
    assert!(attachment.lease().profile_identity().is_none());
}
#[test]
fn t856_adb_adapter_preserves_saved_profiles_and_unknown_identity_policy() {
    let (attachment, _) = attachment();
    attachment.begin_with_transport(
        Some("device:old-profile".into()),
        Some(blent_config::adb::Transport::Usb),
    );
    assert_eq!(
        attachment.lease().profile_identity(),
        Some(("old-profile".into(), "usb"))
    );
    attachment.begin_with_transport(
        Some("device:old-profile".into()),
        Some(blent_config::adb::Transport::Network),
    );
    assert_eq!(
        attachment.lease().profile_identity(),
        Some(("old-profile".into(), "network"))
    );
    for name in ["transport:serial", "usb:serial", "device:"] {
        attachment.begin_with_transport(Some(name.into()), Some(blent_config::adb::Transport::Usb));
        assert!(attachment.lease().profile_identity().is_none());
    }
    attachment.begin(Some("unclassified".into()));
    assert!(attachment.lease().transport().is_none());
    assert!(attachment.lease().profile_identity().is_none());
    attachment.begin_with_transport(None, Some(blent_config::adb::Transport::Usb));
    assert!(attachment.lease().transport().is_none());
}
#[test]
fn t856_invalid_observed_id_retires_instead_of_preserving_previous_authority() {
    let (attachment, _) = attachment();
    for name in [
        "".into(),
        "x".repeat(1025),
        "bad\0identity".into(),
        "device:\n".into(),
    ] {
        attachment.begin_with_transport(
            Some("device:valid".into()),
            Some(blent_config::adb::Transport::Usb),
        );
        let old = attachment.lease();
        attachment.begin_with_transport(Some(name), Some(blent_config::adb::Transport::Usb));
        assert!(!old.apply(|| panic!("old metadata remained current")));
        assert!(attachment.lease().profile_identity().is_none());
        assert!(attachment.lease().transport().is_none());
    }
}
#[tokio::test]
async fn t856_route_replacement_retires_pending_io_before_another_poll() {
    let (attachment, _) = attachment();
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    attachment.begin_route(Some(Identity::authenticated(peer.clone())), Some(Link::Usb));
    let lease = attachment.lease();
    lease.run(async { Ok(()) }).await.unwrap();
    let polls = std::sync::atomic::AtomicUsize::new(0);
    let work = std::future::poll_fn(|_| {
        polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::task::Poll::<anyhow::Result<()>>::Pending
    });
    let pending = lease.run(work);
    tokio::pin!(pending);
    assert!(futures_util::poll!(&mut pending).is_pending());
    assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 1);
    attachment.begin_route(Some(Identity::authenticated(peer)), Some(Link::Network));
    tokio::time::timeout(std::time::Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 1);
    lease
        .run(async { panic!("retired route polled new work") })
        .await
        .unwrap();
}
