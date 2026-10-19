//! Shared terminal-outcome boundary for portable controller supervision.
use anyhow::Result;
use futures_util::FutureExt;
use std::{any::Any, future::Future, panic::AssertUnwindSafe};

/// Includes factory construction and polling. No automatic replay: the owner
/// publishes the result and waits for an explicit subsequent command.
pub(crate) async fn run<F, Fut>(factory: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    AssertUnwindSafe(async { factory().await })
        .catch_unwind()
        .await
        .map_err(panic_error)?
}

/// Last boundary for runtime/supervisor destruction, including a future whose
/// destructor panics outside its poll. Native aborts remain process-wide.
pub(crate) fn worker(action: impl FnOnce(), failed: impl FnOnce(anyhow::Error)) {
    if let Err(panic) = std::panic::catch_unwind(AssertUnwindSafe(action)) {
        failed(panic_error(panic));
    }
}

/// Observe caller-supplied signals without owning their OS adapters. A signal
/// result never substitutes for the original workflow's actual settlement.
pub(crate) async fn with_signals(
    mut work: std::pin::Pin<&mut impl Future<Output = Result<()>>>,
    stop: &tokio::sync::watch::Sender<bool>,
    interrupt: impl Future<Output = std::io::Result<()>>,
    terminate: impl Future<Output = ()>,
) -> Result<()> {
    let mut interrupt_wait = std::pin::pin!(interrupt);
    let mut terminate_wait = std::pin::pin!(terminate);
    tokio::select! {
        result = &mut work => result,
        signal = &mut interrupt_wait => finish_signal(stop, work, signal).await,
        _ = &mut terminate_wait => finish_signal(stop, work, Ok(())).await,
    }
}
async fn finish_signal(
    stop: &tokio::sync::watch::Sender<bool>,
    work: std::pin::Pin<&mut impl Future<Output = Result<()>>>,
    signal: std::io::Result<()>,
) -> Result<()> {
    stop.send_replace(true);
    let result = work.await;
    signal?;
    result
}

#[cfg(test)]
mod signal_tests;
fn panic_error(panic: Box<dyn Any + Send>) -> anyhow::Error {
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic payload");
    anyhow::anyhow!("Control worker panicked: {message}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t812_outer_boundary_handles_return_unwind_and_unknown_payload() {
        worker(|| {}, |_| panic!("successful return reported failure"));
        for payload in [0, 1, 2] {
            let mut failure = None;
            worker(
                || match payload {
                    0 => panic!("literal fixture"),
                    1 => std::panic::panic_any("owned fixture".to_string()),
                    _ => std::panic::panic_any(123u32),
                },
                |error| failure = Some(error.to_string()),
            );
            assert!(failure.unwrap().starts_with("Control worker panicked:"));
        }
    }
}
