//! T977: implement the portable port without OS handles or domain implementation.
use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct Port {
    requested: Mutex<Option<(String, AudioProfile)>>,
    admitted: AtomicBool,
    lost: AtomicBool,
    closing: AtomicBool,
    retired: AtomicBool,
}
impl Invitation<AudioProfile> for Port {
    fn invite<'a>(
        &'a self,
        credential: &'a str,
        profile: &'a AudioProfile,
    ) -> InvitationResult<'a> {
        Box::pin(std::future::poll_fn(move |_| {
            *self.requested.lock().unwrap() = Some((credential.to_owned(), *profile));
            if self.admitted.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }))
    }
}
impl Connection<AudioProfile> for Port {
    fn monitor(&self) -> InvitationResult<'_> {
        Box::pin(std::future::poll_fn(move |_| {
            if self.lost.load(Ordering::SeqCst) {
                Poll::Ready(Err(anyhow::anyhow!("original connection lost")))
            } else {
                Poll::Pending
            }
        }))
    }
    fn close(&self) -> InvitationResult<'_> {
        self.closing.store(true, Ordering::SeqCst);
        Box::pin(std::future::poll_fn(move |_| {
            if self.retired.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }))
    }
}

#[test]
fn t977_invitation_cancellation_and_cleanup_retain_original_independent_authority() {
    let original = Port::default();
    let other = Port::default();
    let profile = AudioProfile::new(Direction::Speakers);
    let credential = "a".repeat(64);
    let mut context = Context::from_waker(Waker::noop());
    let contract: &dyn Connection<AudioProfile> = &original;
    let mut invitation = contract.invite(&credential, &profile);
    assert!(invitation.as_mut().poll(&mut context).is_pending());
    assert_eq!(
        *original.requested.lock().unwrap(),
        Some((credential.clone(), profile))
    );
    assert!(other.requested.lock().unwrap().is_none());
    drop(invitation); // Cancellation is not native/route retirement.
    assert!(!original.retired.load(Ordering::SeqCst));
    assert!(!original.admitted.load(Ordering::SeqCst));
    let mut cleanup = contract.close();
    assert!(cleanup.as_mut().poll(&mut context).is_pending());
    drop(cleanup); // A dropped bounded wait also is not retirement proof.
    assert!(original.closing.load(Ordering::SeqCst));
    assert!(!other.closing.load(Ordering::SeqCst));
    assert!(!original.retired.load(Ordering::SeqCst));
    original.retired.store(true, Ordering::SeqCst);
    assert!(matches!(
        contract.close().as_mut().poll(&mut context),
        Poll::Ready(Ok(()))
    ));
}

#[test]
fn t977_acceptance_and_monitoring_do_not_grant_media_or_retire_other_direction() {
    let microphone = Port::default();
    let speaker = Port::default();
    microphone.admitted.store(true, Ordering::SeqCst);
    let mut context = Context::from_waker(Waker::noop());
    let profile = AudioProfile::new(Direction::Microphone);
    let credential = "b".repeat(64);
    assert!(matches!(
        microphone
            .invite(&credential, &profile)
            .as_mut()
            .poll(&mut context),
        Poll::Ready(Ok(()))
    ));
    assert!(microphone
        .monitor()
        .as_mut()
        .poll(&mut context)
        .is_pending());
    microphone.lost.store(true, Ordering::SeqCst);
    assert!(matches!(
        microphone.monitor().as_mut().poll(&mut context),
        Poll::Ready(Err(_))
    ));
    assert!(speaker.monitor().as_mut().poll(&mut context).is_pending());
    assert!(!microphone.retired.load(Ordering::SeqCst));
    assert!(!speaker.retired.load(Ordering::SeqCst));
}
