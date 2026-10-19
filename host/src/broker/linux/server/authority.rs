//! One process-owned admission domain, independent of broker attempt lifetime.
use anyhow::Result;
use blent_config::retirement::Pool;
use std::sync::OnceLock;

pub(super) fn shared() -> Result<&'static Pool> {
    static POOL: OnceLock<Result<Pool, String>> = OnceLock::new();
    cached(&POOL, || {
        let capacity =
            blent_config::retirement_limits::configured("BLENT_USB_AUTHORITY_WORKERS", 1, 4096)?;
        Ok(Pool::new(capacity)?)
    })
}

fn cached(
    cell: &OnceLock<Result<Pool, String>>,
    initialize: impl FnOnce() -> Result<Pool>,
) -> Result<&Pool> {
    cell.get_or_init(|| initialize().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error}"))
}

#[cfg(test)]
pub(in crate::broker::linux) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1100_cache_preserves_original_pool_and_failed_initialization() {
        let cell = OnceLock::new();
        let original = cached(&cell, || Ok(Pool::new(1)?)).unwrap();
        let admission = original.reserve().unwrap();
        let current = cached(&cell, || panic!("must not replace original pool")).unwrap();
        assert!(std::ptr::eq(original, current));
        assert!(current.reserve().is_err());
        drop(admission);
        assert_eq!(current.active(), 0);
        let failed = OnceLock::new();
        assert!(cached(&failed, || anyhow::bail!("fixture initialization failed")).is_err());
        let retry = cached(&failed, || {
            panic!("must not replace failed process context")
        });
        assert_eq!(
            retry.unwrap_err().to_string(),
            "fixture initialization failed"
        );
    }
}
