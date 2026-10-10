//! Process-entry runtime ownership. All resource supervisors finish first.
use std::future::Future;
pub fn run<T>(work: impl Future<Output = T>) -> std::io::Result<T> {
    let budget =
        blent_config::retirement_limits::configured("BLENT_RUNTIME_STOP_MS", 1000, 60_000)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(work);
    runtime.shutdown_timeout(std::time::Duration::from_millis(budget as u64));
    Ok(result)
}
