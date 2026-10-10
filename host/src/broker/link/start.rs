//! Native startup and teardown share one admission, including cancelled waits.
use crate::accessory::{Owner, Resource};
use anyhow::Result;
use blent_config::retirement::{Admission, Completion, Observe};
use std::{future::Future, pin::Pin};

struct Completed(Completion);
impl Observe for Completed {
    fn poll_complete(&mut self) -> bool {
        self.0.is_complete()
    }
}
struct Retained {
    owner: Option<Box<dyn Owner>>,
    native: Completion,
    admission: Option<Admission>,
    completion: Completion,
}
impl Drop for Retained {
    fn drop(&mut self) {
        self.owner.take();
        if let Some(admission) = self.admission.take() {
            admission.retain(Completed(self.native.clone()));
        }
    }
}
impl Owner for Retained {
    fn completion(&self) -> Completion {
        self.completion.clone()
    }
    fn retire(mut self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            let result = self.owner.take().unwrap().retire().await;
            if self.native.is_complete() {
                self.admission.take();
            }
            result
        })
    }
}
pub(super) fn open(
    admission: Admission,
    factory: impl FnOnce() -> Result<Resource>,
) -> Result<Resource> {
    let mut resource = factory()?;
    resource.owner = Box::new(Retained {
        native: resource.owner.completion(),
        completion: admission.completion(),
        owner: Some(resource.owner),
        admission: Some(admission),
    });
    Ok(resource)
}
