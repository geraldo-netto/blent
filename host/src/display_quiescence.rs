//! Independent display stop/admission handshake. Receipts attest adapter-verified
//! original native cleanup; commands and task completion are never such evidence.
use anyhow::{ensure, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

#[derive(Clone)]
struct Request {
    identity: Arc<()>,
    open: bool,
}
struct Owner {
    requests: watch::Sender<Request>,
    receipts: Vec<watch::Receiver<Option<Arc<()>>>>,
}
/// Clones address one authoritative request owner. The current display assignment
/// alone may command it; producers receive only read views and their own receipt.
#[derive(Clone)]
pub struct Control(Arc<Mutex<Owner>>);
#[derive(Clone)]
pub struct Source(watch::Receiver<Request>);
#[derive(Clone)]
pub struct Permit(Arc<()>);
pub struct Pause(Arc<()>);
pub struct Participant {
    source: Source,
    receipt: watch::Sender<Option<Arc<()>>>,
}
pub struct Fence {
    pause: Arc<()>,
    source: Source,
    receipts: Vec<watch::Receiver<Option<Arc<()>>>>,
}

/// Fixed participants must be admitted before any native startup is exposed.
/// Linux uses capture plus selector, or capture alone for the in-process encoder.
pub fn channel(participants: usize) -> Result<(Control, Vec<Participant>)> {
    ensure!(
        (1..=2).contains(&participants),
        "invalid display quiescence capacity"
    );
    let (requests, source) = watch::channel(Request {
        identity: Arc::new(()),
        open: true,
    });
    let mut receipts = Vec::with_capacity(participants);
    let mut producers = Vec::with_capacity(participants);
    for _ in 0..participants {
        let (receipt, observed) = watch::channel(None);
        receipts.push(observed);
        producers.push(Participant {
            source: Source(source.clone()),
            receipt,
        });
    }
    Ok((
        Control(Arc::new(Mutex::new(Owner { requests, receipts }))),
        producers,
    ))
}
impl Control {
    pub fn pause(&self) -> Fence {
        let owner = self.0.lock().unwrap();
        if owner.requests.borrow().open {
            owner.requests.send_replace(Request {
                identity: Arc::new(()),
                open: false,
            });
        }
        let pause = owner.requests.borrow().identity.clone();
        Fence {
            pause,
            source: Source(owner.requests.subscribe()),
            receipts: owner.receipts.clone(),
        }
    }
    /// Only this exact current stop, with every original producer settled, can
    /// authorize another work generation. A foreign/stale fence never resumes it.
    pub fn resume(&self, fence: &Fence) -> bool {
        let owner = self.0.lock().unwrap();
        let current = owner.requests.borrow().clone();
        if current.open || !Arc::ptr_eq(&current.identity, &fence.pause) || !fence.complete() {
            return false;
        }
        owner.requests.send_replace(Request {
            identity: Arc::new(()),
            open: true,
        });
        true
    }
}
impl Source {
    pub fn begin(&self) -> Option<Permit> {
        let request = self.0.borrow();
        (request.open && self.0.has_changed().is_ok()).then(|| Permit(request.identity.clone()))
    }
    pub fn permits(&self, original: &Permit) -> bool {
        self.begin()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &original.0))
    }
    pub fn changed(&self) -> crate::capture_contract::Signal<'static> {
        let mut source = self.0.clone();
        let open = source.has_changed().is_ok();
        source.borrow_and_update();
        Box::pin(async move {
            if open {
                let _ = source.changed().await;
            } else {
                std::future::pending::<()>().await;
            }
        })
    }
}
impl Participant {
    pub fn source(&self) -> Source {
        self.source.clone()
    }
    pub fn pending(&self) -> Option<Pause> {
        let request = self.source.0.borrow();
        let acknowledged = self
            .receipt
            .borrow()
            .as_ref()
            .is_some_and(|seen| Arc::ptr_eq(seen, &request.identity));
        (!request.open && !acknowledged && self.source.0.has_changed().is_ok())
            .then(|| Pause(request.identity.clone()))
    }
    /// Adapter boundary: call only after exact original creators/native workers
    /// have retired and any retained-helper writer pause has been acknowledged.
    /// Each producer owns one receipt; dropping it fails closed, even after ACK.
    pub fn acknowledge(&self, original: &Pause) -> bool {
        let Some(current) = self.pending() else {
            return false;
        };
        if !Arc::ptr_eq(&current.0, &original.0) {
            return false;
        }
        self.receipt.send_replace(Some(original.0.clone()));
        true
    }
}
impl Fence {
    pub fn complete(&self) -> bool {
        let request = self.source.0.borrow();
        if request.open
            || !Arc::ptr_eq(&request.identity, &self.pause)
            || self.source.0.has_changed().is_err()
        {
            return false;
        }
        self.receipts.iter().all(|receipt| {
            receipt.has_changed().is_ok()
                && receipt
                    .borrow()
                    .as_ref()
                    .is_some_and(|seen| Arc::ptr_eq(seen, &self.pause))
        })
    }
}

#[cfg(test)]
mod tests;
