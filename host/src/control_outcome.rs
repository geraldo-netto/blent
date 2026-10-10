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
