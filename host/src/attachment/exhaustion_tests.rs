//! T1175: independent numeric boundaries; old leases never become current again.
use super::*;

fn seeded(value: u64) -> (Attachment, watch::Receiver<EncoderSettings>) {
    let (settings, receiver) = watch::channel(EncoderSettings {
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
    });
    let attachment = Attachment::new(settings);
    attachment.0.state.lock().unwrap().generation = value;
    attachment.0.generation.send_replace(value);
    (attachment, receiver)
}

#[test]
fn t1175_legacy_handoff_exhaustion_cannot_revive_zero_or_current_leases() {
    let (attachment, settings) = seeded(0);
    let zero = attachment.lease();
    attachment.0.state.lock().unwrap().generation = u64::MAX;
    attachment.0.generation.send_replace(u64::MAX);
    let original = attachment.lease();
    attachment
        .0
        .settings
        .send_modify(|settings| settings.geometry_ready = true);
    let mut changed = attachment.generation_updates();
    changed.borrow_and_update();
    attachment.begin_route(None, None);
    assert!(!zero.apply(|| panic!("T1175: stale zero lease revived")));
    assert!(!original.apply(|| panic!("T1175: exhausted original lease remained current")));
    assert!(attachment.0.state.lock().unwrap().closed);
    assert_eq!(*changed.borrow(), u64::MAX);
    assert!(changed.has_changed().unwrap());
    assert!(!settings.borrow().geometry_ready);
    assert!(!*attachment.subscribe().borrow());
    assert!(attachment.token().is_err());
    attachment.begin_route(None, None);
    assert_eq!(*attachment.generation_updates().borrow(), u64::MAX);
}

#[test]
fn t1175_retirement_pins_maximum_without_overflow_or_lease_revival() {
    for value in [0, 1, u64::MAX - 1, u64::MAX] {
        let (attachment, _) = seeded(value);
        let original = attachment.lease();
        let mut changed = attachment.generation_updates();
        changed.borrow_and_update();
        attachment.retire();
        assert_eq!(*changed.borrow(), value.saturating_add(1));
        assert!(changed.has_changed().unwrap());
        assert!(!original.apply(|| panic!("T1175: retired original admitted work")));
        attachment.retire();
        assert_eq!(*changed.borrow(), value.saturating_add(1));
        assert!(attachment.send(true).is_err());
    }
}

#[tokio::test]
async fn t1175_last_valid_handoff_admits_once_and_notifies_terminal_watchers() {
    let (attachment, _) = seeded(u64::MAX - 1);
    attachment.begin_route(None, None);
    let last = attachment.lease();
    assert_eq!(*attachment.generation_updates().borrow(), u64::MAX);
    assert!(last.apply(|| {}));
    let mut retirement = last.retirement();
    attachment.begin_route(None, None);
    assert!(!last.apply(|| panic!("T1175: exhaustion admitted replacement")));
    assert!(attachment.0.state.lock().unwrap().closed);
    tokio::time::timeout(std::time::Duration::from_secs(1), retirement.retired())
        .await
        .unwrap();
}

#[test]
fn t1175_bounded_handoffs_preserve_the_independent_u128_boundary_model() {
    for seed in (0..=64).chain(u64::MAX - 64..=u64::MAX) {
        let (attachment, _) = seeded(seed);
        for step in 1..=70u128 {
            let before = attachment.lease();
            attachment.begin_route(None, None);
            let mathematical = u128::from(seed) + step;
            let expected = mathematical.min(u128::from(u64::MAX)) as u64;
            let state = attachment.0.state.lock().unwrap();
            assert_eq!(state.generation, expected);
            assert_eq!(state.closed, mathematical > u128::from(u64::MAX));
            drop(state);
            assert!(!before.apply(|| panic!("T1175: prior lease survived handoff")));
        }
    }
}

#[test]
fn t1175_poisoned_original_action_can_only_retire_not_revive() {
    let (attachment, _) = seeded(u64::MAX);
    let original = attachment.lease();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        original.apply(|| panic!("T1175: injected original action failure"));
    }));
    assert!(failure.is_err());
    attachment.retire();
    assert!(!attachment.0.state.is_poisoned());
    assert!(!original.apply(|| panic!("T1175: poisoned owner revived")));
    assert_eq!(*attachment.generation_updates().borrow(), u64::MAX);
}
