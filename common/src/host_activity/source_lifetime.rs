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
        self.0
            .native
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(completion);
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
