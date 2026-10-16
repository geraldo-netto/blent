//! Read-only current-work checks around polling. Callers arm original source
//! notices and retain native owners independently; this grants no authority.
use std::future::Future;

pub async fn guarded<T>(current: impl Fn() -> bool, work: impl Future<Output = T>) -> Option<T> {
    tokio::pin!(work);
    std::future::poll_fn(|cx| {
        if !current() {
            return std::task::Poll::Ready(None);
        }
        let result = work.as_mut().poll(cx);
        if !current() {
            return std::task::Poll::Ready(None);
        }
        result.map(Some)
    })
    .await
}
