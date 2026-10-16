//! Capture native observations survive parent results and creator destruction.
use crate::capture_contract::NativeRetirement;
use blent_config::retirement::Completion;
use std::sync::{Arc, Mutex, Weak};

struct Recorder(Arc<Mutex<Vec<Completion>>>);

#[derive(Clone)]
pub(crate) struct Observation {
    creator: Weak<Recorder>,
    native: Arc<Mutex<Vec<Completion>>>,
}

pub(crate) fn channel() -> (Arc<dyn NativeRetirement>, Observation) {
    let native = Arc::new(Mutex::new(Vec::new()));
    let creator = Arc::new(Recorder(native.clone()));
    let observation = Observation {
        creator: Arc::downgrade(&creator),
        native,
    };
    (creator, observation)
}

impl NativeRetirement for Recorder {
    fn retain(&self, original: Completion) {
        let mut native = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Each original native owner registers once; completed history must not
        // accumulate through repeated encoder/helper restarts.
        native.retain(|completion| !completion.is_complete());
        native.push(original);
    }
}

impl Observation {
    pub(crate) fn complete(&self) -> bool {
        if self.creator.upgrade().is_some() {
            return false;
        }
        self.native
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .all(Completion::is_complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blent_config::retirement::Pool;

    #[test]
    fn t1026_creator_and_original_native_completion_are_independent() {
        let (report, observed) = channel();
        let creator = report.clone();
        let native = Pool::new(1).unwrap();
        let original = native.reserve().unwrap();
        report.retain(original.completion());
        assert!(!observed.complete());
        drop(report);
        assert!(!observed.complete());
        drop(creator);
        assert!(!observed.complete());
        drop(original);
        assert!(observed.complete());
    }

    #[test]
    fn t1026_completed_history_is_replaced_without_losing_pending_native() {
        let native = Pool::new(2).unwrap();
        let (report, observed) = channel();
        let pending = native.reserve().unwrap();
        report.retain(pending.completion());
        for _ in 0..64 {
            let old = native.reserve().unwrap();
            report.retain(old.completion());
            drop(old);
            assert!(observed.native.lock().unwrap().len() <= 2);
        }
        drop(report);
        assert!(!observed.complete());
        drop(pending);
        assert!(observed.complete());
    }
}
