//! T1086: independent target oracles; no OS, ADB, consent or native resources.
use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};

fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let Poll::Ready(result) = poll(future.as_mut()) else {
        panic!("T1086 unexpectedly pending")
    };
    result
}
async fn unexpected() -> Result<String> {
    panic!("T1086 unexpected ADB discovery")
}

#[test]
fn t1086_original_target_survives_auto_selection_change_and_new_intent_can_reselect() {
    let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
    let a = ready(intent.bind(|| async { Ok("tablet-A".into()) })).unwrap();
    let b = ready(intent.bind(|| async { Ok("tablet-B".into()) })).unwrap();
    assert!(std::ptr::eq(a, b));
    assert!(std::ptr::eq(a, ready(intent.bind(unexpected)).unwrap()));
    assert_eq!(
        b.target(),
        Target::Adb(Some("tablet-A")),
        "T1086 automatic selection changed original"
    );
    let fresh = OriginalTarget::new(Target::Adb(None)).unwrap();
    assert_eq!(
        ready(fresh.bind(|| async { Ok("tablet-B".into()) }))
            .unwrap()
            .target(),
        Target::Adb(Some("tablet-B"))
    );
}

#[test]
fn t1086_explicit_adb_and_usb_never_discover_or_fallback() {
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    let usb = OriginalTarget::new(Target::Usb(peer.clone())).unwrap();
    assert_eq!(
        ready(usb.bind(unexpected)).unwrap().target(),
        Target::Usb(peer)
    );
    for serial in [
        "original",
        " whitespace ",
        "$(touch never); --serial=other",
        "é",
    ] {
        let explicit = OriginalTarget::new(Target::Adb(Some(serial))).unwrap();
        assert_eq!(
            ready(explicit.bind(unexpected)).unwrap().target(),
            Target::Adb(Some(serial))
        );
    }
    assert!(OriginalTarget::new(Target::Adb(Some(""))).is_err());
}

#[test]
fn t1086_rejected_or_cancelled_lookup_publishes_no_original() {
    let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
    assert!(ready(intent.bind(|| async { anyhow::bail!("missing original") })).is_err());
    assert!(ready(intent.bind(|| async { Ok(String::new()) })).is_err());
    let pending = intent.bind(std::future::pending::<Result<String>>);
    let mut pending = Box::pin(pending);
    assert!(poll(pending.as_mut()).is_pending());
    drop(pending);
    assert_eq!(
        ready(intent.bind(|| async { Ok("first-effective".into()) }))
            .unwrap()
            .target(),
        Target::Adb(Some("first-effective"))
    );
}

#[test]
fn t1086_late_or_concurrent_resolution_returns_only_committed_original() {
    use std::sync::atomic::{AtomicBool, Ordering};
    for first in ["tablet-A", "tablet-B"] {
        let intent = OriginalTarget::new(Target::Adb(None)).unwrap();
        let released = AtomicBool::new(false);
        let mut late = Box::pin(intent.bind(|| {
            std::future::poll_fn(|_| {
                if released.load(Ordering::Acquire) {
                    Poll::Ready(Ok("late-other".into()))
                } else {
                    Poll::Pending
                }
            })
        }));
        assert!(poll(late.as_mut()).is_pending());
        let committed = ready(intent.bind(|| async { Ok(first.into()) })).unwrap();
        released.store(true, Ordering::Release);
        let Poll::Ready(Ok(late)) = poll(late.as_mut()) else {
            panic!("T1086 missing late result")
        };
        assert!(std::ptr::eq(committed, late));
        assert_eq!(late.target(), Target::Adb(Some(first)));
    }
}

#[test]
fn t1086_bounded_serial_values_remain_exact_without_hidden_normalization() {
    for length in 1..=128 {
        for byte in 0..=127u8 {
            let serial = String::from_utf8(vec![byte; length]).unwrap();
            let intent = OriginalTarget::new(Target::Adb(Some(&serial))).unwrap();
            assert_eq!(
                ready(intent.bind(unexpected)).unwrap().target(),
                Target::Adb(Some(serial.as_str()))
            );
        }
    }
}

#[test]
fn t1086_concurrent_native_free_callers_share_one_committed_original() {
    use std::sync::{Arc, Barrier};
    for _ in 0..16 {
        let intent = Arc::new(OriginalTarget::new(Target::Adb(None)).unwrap());
        let gate = Arc::new(Barrier::new(2));
        let a = racing(intent.clone(), gate.clone(), "tablet-A");
        let b = racing(intent, gate, "tablet-B");
        let a = a.join().unwrap();
        let b = b.join().unwrap();
        assert_eq!(
            a, b,
            "T1086 concurrent callers obtained different originals"
        );
        assert!(["tablet-A", "tablet-B"].contains(&a.as_str()));
    }
}
fn racing(
    intent: std::sync::Arc<OriginalTarget>,
    gate: std::sync::Arc<std::sync::Barrier>,
    serial: &'static str,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let original = ready(intent.bind(|| async {
            gate.wait();
            Ok(serial.into())
        }))
        .unwrap();
        let Target::Adb(Some(serial)) = original.target() else {
            panic!("T1086 unexpected target")
        };
        serial.to_owned()
    })
}
