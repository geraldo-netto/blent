use super::*;
use blent_config::connection::pairing::PeerId;
fn peer(byte: u8) -> PeerId {
    PeerId::parse(format!("{byte:02x}").repeat(32)).unwrap()
}
#[tokio::test]
async fn t917_usb_preempts_adb_only_after_owned_cleanup_and_rejects_stale_retirement() {
    let mut state = super::super::tests::fixture();
    state.slots = Slots::new(1).unwrap();
    let serial = "fake:t917".to_owned();
    state.change_primary(&Some(serial.clone()));
    state.prepare_primary();
    let ticket = state
        .slots
        .ticket(&DisplayOwner::Adb(serial.clone()))
        .unwrap();
    let old_lease = state.config.tablet.lease();
    state.slots.select(Some(peer(1)));
    assert_eq!(state.slots.plan(), DisplayPlan::Replace(ticket));
    state.preempt_for_usb();
    assert!(state.current.is_none());
    assert!(!old_lease.apply(|| ()));
    assert_eq!(state.slots.plan(), DisplayPlan::Wait);
    state.change_primary(&Some(serial.clone()));
    state.prepare_primary();
    assert!(state.pending.is_empty());
    let retired = state.retiring.join_next().await.unwrap().unwrap();
    state.retired((retired.0.clone(), 999, retired.2));
    assert_eq!(state.slots.plan(), DisplayPlan::Wait);
    state.retired(retired.clone());
    assert_eq!(state.slots.plan(), DisplayPlan::Ready(0));
    let usb = state.slots.reserve(DisplayOwner::Peer(peer(1)), 0).unwrap();
    state.retired(retired);
    assert_eq!(state.slots.plan(), DisplayPlan::Current(usb));
    let native_lease = state.config.tablet.lease();
    state.change_primary(&Some("different-adb".into()));
    assert!(
        native_lease.apply(|| ()),
        "T917 ADB discovery invalidated native display authority"
    );
    state.prepare_primary();
    assert!(state.pending.is_empty());
    state.slots.close(usb).unwrap();
    state.slots.select(None);
    assert!(!state.slots.adb_available(0));
    state.slots.retired(usb).unwrap();
    state.change_primary(&Some(serial.clone()));
    state.prepare_primary();
    assert!(state.pending.contains_key(&serial));
    state.stop().await;
}
#[tokio::test]
async fn t917_free_usb_slot_preserves_adb_primary() {
    let mut state = super::super::tests::fixture();
    state.change_primary(&Some("fake:t917".into()));
    state.prepare_primary();
    state.slots.select(Some(peer(1)));
    state.preempt_for_usb();
    assert_eq!(state.current.as_deref(), Some("fake:t917"));
    assert_eq!(state.slots.plan(), DisplayPlan::Ready(1));
    assert_eq!(state.available_slot(), None);
    state.prepare_extra("other".into()).await;
    assert!(!state.pending.contains_key("other"));
    state.slots.select(None);
    assert_eq!(state.available_slot(), Some(1));
    state.stop().await;
}
#[tokio::test]
async fn t917_full_capacity_retires_extra_without_invalidating_primary() {
    let mut state = super::super::tests::fixture();
    let primary = "fake:t917-primary".to_owned();
    state.change_primary(&Some(primary.clone()));
    state.prepare_primary();
    let primary_ticket = state
        .slots
        .ticket(&DisplayOwner::Adb(primary.clone()))
        .unwrap();
    let lease = state.config.tablet.lease();
    state.retired((primary.clone(), 0, primary_ticket)); // Active is not retired.
    assert_eq!(
        state.slots.owner(primary_ticket),
        Some(&DisplayOwner::Adb(primary.clone()))
    );
    let extra = "fake:t917-extra".to_owned();
    state
        .slots
        .reserve(DisplayOwner::Adb(extra.clone()), 1)
        .unwrap();
    let runtime = session::Spec {
        capture: capture::CaptureConfig {
            instance: 1,
            helper_path: "/missing-t917-helper".into(),
            ..Default::default()
        },
        ports: (0, 0),
        token: None,
        devices: (false, false, false),
    }
    .prepare(state.config.extra.mode_tx.clone())
    .start(state.config.extra.shutdown_rx.clone())
    .await
    .unwrap();
    state.pending.insert(
        extra.clone(),
        Pending {
            instance: 1,
            session: Some(runtime),
        },
    );
    state.slots.select(Some(peer(1)));
    state.preempt_for_usb();
    assert!(!state.pending.contains_key(&extra));
    assert_eq!(state.current, Some(primary));
    assert!(lease.apply(|| ()));
    state.preempt_for_usb();
    assert_eq!(state.retiring.len(), 1);
    let retired = state.retiring.join_next().await.unwrap().unwrap();
    state.retired(retired);
    assert_eq!(state.slots.plan(), DisplayPlan::Ready(1));
    assert!(lease.apply(|| ()));
    state.stop().await;
}
