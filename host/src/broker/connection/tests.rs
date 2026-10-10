use super::*;
use blent_config::{
    connection::pairing::PeerId,
    retirement::{Admission, Observe, Pool},
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
struct Native {
    admission: Admission,
    stopped: Arc<AtomicBool>,
}
struct Pending(Arc<AtomicBool>);
impl Observe for Pending {
    fn poll_complete(&mut self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
impl Owner for Native {
    fn completion(&self) -> Completion {
        self.admission.completion()
    }
    fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            let Native { admission, stopped } = *self;
            if stopped.load(Ordering::SeqCst) {
                drop(admission);
                Ok(())
            } else {
                admission.retain(Pending(stopped));
                anyhow::bail!("fixture native close pending")
            }
        })
    }
}
fn candidate() -> (Verified, Pairing, Lease, Arc<AtomicBool>) {
    let mut pairing = Pairing::default();
    let peer = PeerId::parse("ab".repeat(32)).unwrap();
    let ticket = pairing.begin(peer.clone(), true, 0, 100).unwrap();
    let lease = pairing.authenticated(&ticket, &peer, 1).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let owner = Native {
        admission: Pool::new(1).unwrap().reserve().unwrap(),
        stopped: stopped.clone(),
    };
    let (stream, _remote) = tokio::io::duplex(8);
    (
        Verified {
            resource: Resource {
                stream: Box::new(stream),
                owner: Box::new(owner),
            },
            ticket,
            peer,
        },
        pairing,
        lease,
        stopped,
    )
}
#[tokio::test]
async fn t872_failed_native_retirement_does_not_release_link_ownership() {
    let (verified, pairing, lease, stopped) = candidate();
    let mut connection = Connection::new(verified, &pairing, &lease, Limits::default()).unwrap();
    assert!(!connection.completion().is_complete());
    assert!(connection.retire().await.is_err());
    assert!(connection.retire().await.is_err());
    assert!(!connection.completion().is_complete());
    stopped.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), async {
        while !connection.completion().is_complete() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    connection.retire().await.unwrap();
    assert!(connection.session().capabilities().is_none());
}
#[tokio::test]
async fn t872_verified_setup_rechecks_current_pairing_before_install() {
    for mode in 0..4 {
        let (mut verified, mut pairing, lease, stopped) = candidate();
        stopped.store(true, Ordering::SeqCst);
        let mut limits = Limits::default();
        match mode {
            0 => {
                pairing.cancel(&lease.ticket);
            }
            1 => {
                verified.ticket.epoch += 1;
            }
            2 => {
                verified.peer = PeerId::parse("cd".repeat(32)).unwrap();
            }
            _ => limits.channel_bytes = 0,
        }
        assert!(Connection::new(verified, &pairing, &lease, limits).is_err());
    }
    let (verified, pairing, lease, stopped) = candidate();
    stopped.store(true, Ordering::SeqCst);
    let mut connection = Connection::new(verified, &pairing, &lease, Limits::default()).unwrap();
    connection.retire().await.unwrap();
    assert!(connection.completion().is_complete());
}
#[cfg(target_os = "linux")]
#[tokio::test]
async fn t872_native_helper_retirement_observes_actual_child_exit() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let helper = directory.path().join("helper");
    std::fs::write(&helper, b"#!/bin/sh\nexec cat\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let (mut verified, pairing, lease, stopped) = candidate();
    stopped.store(true, Ordering::SeqCst);
    // Native owner fixture only; stock TLS/session duplex has its own regression.
    verified.resource = crate::accessory::linux::open(
        &helper,
        &crate::accessory::linux::Target::new(1, &[1]).unwrap(),
    )
    .unwrap();
    let completion = verified.resource.owner.completion();
    let mut connection = Connection::new(verified, &pairing, &lease, Limits::default()).unwrap();
    assert!(!completion.is_complete());
    tokio::time::timeout(Duration::from_secs(3), connection.retire())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_complete());
}
