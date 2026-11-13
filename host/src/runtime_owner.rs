//! Process-entry runtime ownership. All resource supervisors finish first.
use std::future::Future;
pub fn run<T>(work: impl Future<Output = T>) -> std::io::Result<T> {
    let _wifi_retirement = crate::wifi::ProcessExit;
    let budget =
        blent_config::retirement_limits::configured("BLENT_RUNTIME_STOP_MS", 1000, 60_000)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    // Keep the executor available to admitted asynchronous cleanup, including
    // cleanup triggered by error returns, caller cancellation and unwinding.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.block_on(work)));
    runtime.block_on(crate::process_retirement::PROCESS.drain());
    runtime.shutdown_timeout(std::time::Duration::from_millis(budget as u64));
    match result {
        Ok(result) => Ok(result),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
