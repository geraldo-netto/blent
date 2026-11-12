//! Exact dependent certificates outlive task results and lease destructor entry.
use crate::retirement::{Admission, Completion, Observe};
use std::sync::{Arc, Mutex};
#[derive(Clone)]
pub struct SourceLifetime(Arc<Owned>);
struct Owned {
    admission: Option<Admission>,
    native: Mutex<Vec<Completion>>,
}
impl SourceLifetime {
    pub fn new(admission: Admission) -> Self {
        Self(Arc::new(Owned {
            admission: Some(admission),
            native: Mutex::new(Vec::new()),
        }))
    }
    pub fn completion(&self) -> Completion {
        self.0
            .admission
            .as_ref()
            .expect("owned source lifetime")
            .completion()
    }
    /// Register before transferring native ownership or exposing its result.
    pub fn retain(&self, completion: Completion) {
        let mut native = self
            .0
            .native
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Certificates are monotonic. Forget settled attempts, never live debt.
        native.retain(|proof| !proof.is_complete());
        native.push(completion);
    }
}
struct Native(Vec<Completion>);
impl Observe for Native {
    fn poll_complete(&mut self) -> bool {
        self.0.iter().all(Completion::is_complete)
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let native = self
            .native
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.admission
            .take()
            .expect("owned source admission")
            .retain(Native(std::mem::take(native)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retirement::Pool;

    #[test]
    fn t1343_retry_certificates_remain_bounded_without_forgetting_live_native_debt() {
        let owner = SourceLifetime::new(Pool::new(1).unwrap().reserve().unwrap());
        let pool = Pool::new(2).unwrap();
        let original = pool.reserve().unwrap();
        owner.retain(original.completion());
        for _ in 0..256 {
            let attempt = pool.reserve().unwrap();
            owner.retain(attempt.completion());
            assert_eq!(owner.0.native.lock().unwrap().len(), 2);
            drop(attempt);
        }
        let complete = owner.completion();
        drop(owner);
        assert!(!complete.is_complete());
        drop(original);
        let settled = std::time::Instant::now();
        while !complete.is_complete() {
            assert!(settled.elapsed() < std::time::Duration::from_secs(3));
            std::thread::yield_now();
        }
    }
}
