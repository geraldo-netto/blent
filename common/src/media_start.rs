//! Start admission acknowledgement, independent of sampled media status.
//! One bounded token per request; this is not proof of native retirement.
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

#[derive(Clone, Debug)]
pub struct Start(Arc<AtomicU8>);
#[derive(Clone, Debug)]
pub struct Acknowledger(Arc<Settlement>);

#[derive(Debug)]
struct Settlement(Arc<AtomicU8>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Outcome {
    Pending = 0,
    Acknowledged = 1,
    Rejected = 2,
}

impl Drop for Settlement {
    fn drop(&mut self) {
        let _ = self
            .0
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
    }
}

impl Start {
    pub fn pending() -> (Self, Acknowledger) {
        let state = Arc::new(AtomicU8::new(Outcome::Pending as u8));
        (
            Self(state.clone()),
            Acknowledger(Arc::new(Settlement(state))),
        )
    }
    pub fn acknowledged() -> Self {
        Self(Arc::new(AtomicU8::new(Outcome::Acknowledged as u8)))
    }
    pub fn outcome(&self) -> Outcome {
        match self.0.load(Ordering::Acquire) {
            0 => Outcome::Pending,
            1 => Outcome::Acknowledged,
            _ => Outcome::Rejected,
        }
    }
    pub fn is_pending(&self) -> bool {
        self.outcome() == Outcome::Pending
    }
}
impl Acknowledger {
    /// Call after publishing this request's initial status. Acquire observers
    /// must read the backend status after checking this token.
    pub fn acknowledge(&self) {
        self.settle(Outcome::Acknowledged);
    }
    /// Rejection settles admission only, never native resource retirement.
    pub fn reject(&self) {
        self.settle(Outcome::Rejected);
    }
    fn settle(&self, outcome: Outcome) {
        let _ = self
            .0
             .0
            .compare_exchange(0, outcome as u8, Ordering::AcqRel, Ordering::Acquire);
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

#[cfg(test)]
mod rejection_tests {
    use super::*;
    #[test]
    fn t1460_last_request_owner_settles_unconsumed_start() {
        let (start, owner) = Start::pending();
        let clone = owner.clone();
        drop(owner);
        assert!(start.is_pending());
        drop(clone);
        assert!(
            !start.is_pending(),
            "T1460 abandoned request remains pending"
        );
    }
}

#[cfg(test)]
mod outcome_tests {
    use super::*;
    #[test]
    fn t1460_first_settlement_is_final_and_rejection_is_explicit() {
        for accepted in [false, true] {
            let (start, ack) = Start::pending();
            assert_eq!(start.outcome(), Outcome::Pending);
            if accepted {
                ack.acknowledge();
            } else {
                ack.reject();
            }
            ack.reject();
            ack.acknowledge();
            drop(ack);
            let expected = if accepted {
                Outcome::Acknowledged
            } else {
                Outcome::Rejected
            };
            assert_eq!(start.outcome(), expected);
        }
    }
}
