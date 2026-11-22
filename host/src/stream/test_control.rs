//! T1704 fixtures cancel the same owned server future used in production.
use futures_util::future::{AbortHandle, Abortable};

pub(crate) fn controlled(
    work: impl std::future::Future<Output = anyhow::Result<()>>,
) -> (
    AbortHandle,
    impl std::future::Future<Output = anyhow::Result<()>>,
) {
    let (stop, registration) = AbortHandle::new_pair();
    let work = async move {
        match Abortable::new(work, registration).await {
            Ok(outcome) => outcome,
            Err(_) => Ok(()),
        }
    };
    (stop, work)
}
