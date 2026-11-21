//! Start admission acknowledgement, independent of sampled media status.
//! One bounded token per request; this is not proof of native retirement.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone, Debug)]
pub struct Start(Arc<AtomicBool>);
#[derive(Clone, Debug)]
pub struct Acknowledger(Arc<AtomicBool>);

impl Start {
    pub fn pending() -> (Self, Acknowledger) {
        let state = Arc::new(AtomicBool::new(true));
        (Self(state.clone()), Acknowledger(state))
    }
    pub fn acknowledged() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn is_pending(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
impl Acknowledger {
    /// Call after publishing this request's initial status. Acquire observers
    /// must read the backend status after checking this token.
    pub fn acknowledge(&self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1455_acknowledgement_is_sticky_and_request_specific() {
        let (first, ack) = Start::pending();
        let (second, other) = Start::pending();
        assert!(first.is_pending());
        ack.acknowledge();
        ack.acknowledge();
        assert!(!first.clone().is_pending());
        assert!(second.is_pending());
        other.acknowledge();
        assert!(!second.is_pending());
        assert!(!Start::acknowledged().is_pending());
    }
}
