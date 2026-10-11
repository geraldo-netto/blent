use super::*;
use crate::camera_registration::test_support::{authenticated, profile, view, Rig};
use blent_config::connection::{control::State as LinkState, registration_client::Phase};

fn bridge(rig: &Rig) -> Bridge {
    let profile = profile();
    Bridge::owned(
        &profile,
        &"ab".repeat(32),
        rig.owner(&profile, &"ab".repeat(32)).unwrap(),
    )
    .unwrap()
}

#[tokio::test(start_paused = true)]
async fn t933_invitation_supervision_and_actual_retirement_are_independent() {
    let rig = authenticated();
    let bridge = bridge(&rig);
    bridge.invite(&"ab".repeat(32), &profile()).await.unwrap();
    assert!(!bridge.retired());
    rig.client().set(Phase::Active);
    let monitor = bridge.monitor();
    tokio::pin!(monitor);
    assert!(tokio::time::timeout(Duration::from_secs(1), &mut monitor)
        .await
        .is_err());
    // Source replacement with equal counters is independently fatal.
    rig.source.status.lock().unwrap().view = Some(view(&[(1, 2, LinkState::Authenticated)]));
    assert!(monitor.await.is_err());
    assert!(bridge.close().await.is_err());
    assert!(!bridge.retired());
    rig.complete();
    bridge.close().await.unwrap();
    assert!(bridge.retired());
}

#[tokio::test(start_paused = true)]
async fn t933_invitation_wait_and_cancellation_keep_original_owner() {
    let rig = Rig::new(&[]);
    let bridge = bridge(&rig);
    let token = "ab".repeat(32);
    let selected = profile();
    let invitation = bridge.invite(&token, &selected);
    tokio::pin!(invitation);
    assert!(
        tokio::time::timeout(Duration::from_secs(1), &mut invitation)
            .await
            .is_err()
    );
    assert!(rig.calls.lock().unwrap().clients.is_empty());
    assert!(bridge.close().await.is_err());
    assert!(invitation.await.is_err());
    assert!(bridge.monitor().await.is_err());
    rig.source.done.store(true, Ordering::SeqCst);
    bridge.close().await.unwrap();
    assert!(bridge.retired());
}

#[tokio::test]
async fn t933_mismatched_or_replayed_invitation_cannot_expose_new_registration() {
    let rig = authenticated();
    let bridge = bridge(&rig);
    for token in ["ab".repeat(31), "cd".repeat(32), "ab".repeat(33)] {
        assert!(bridge.invite(&token, &profile()).await.is_err());
    }
    let changed = CameraProfile {
        background: true,
        ..profile()
    };
    assert!(bridge.invite(&"ab".repeat(32), &changed).await.is_err());
    assert!(rig.calls.lock().unwrap().clients.is_empty());
    bridge.invite(&"ab".repeat(32), &profile()).await.unwrap();
    assert!(bridge.invite(&"ab".repeat(32), &profile()).await.is_err());
    assert_eq!(rig.calls.lock().unwrap().clients.len(), 1);
    let completion = bridge.admission.as_ref().unwrap().completion();
    drop(bridge);
    assert!(!completion.is_complete());
    rig.complete();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
