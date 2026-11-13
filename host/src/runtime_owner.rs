//! Process-entry runtime ownership. All resource supervisors finish first.
use std::future::Future;

/// Process cutoff rejected new resource admission; this is distinct from I/O failure.
#[derive(Debug)]
pub struct AdmissionClosed;
impl std::fmt::Display for AdmissionClosed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Process resource admission is closed")
    }
}
impl std::error::Error for AdmissionClosed {}

/// Stop new admissions and wake owners; `run` retains the executor until settlement.
pub fn request_shutdown() {
    crate::process_retirement::PROCESS.close();
}

/// Startup checkpoint only; each later admission still checks cutoff atomically.
pub fn check_running() -> std::io::Result<()> {
    crate::process_retirement::PROCESS.check_running()
}

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
