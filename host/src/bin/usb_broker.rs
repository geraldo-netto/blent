#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .try_init()
        .map_err(|_| anyhow::anyhow!("USB diagnostics subscriber unavailable"))?;
    blent::runtime_owner::run(blent::broker::linux::entry::run(std::env::args_os()))?
}
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("USB broker backend is unavailable on this operating system")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn t1236_logging_startup_failure_precedes_native_work() {
        use tracing_subscriber::util::SubscriberInitExt;
        tracing_subscriber::registry().try_init().unwrap();
        assert_eq!(
            super::main().unwrap_err().to_string(),
            "USB diagnostics subscriber unavailable"
        );
    }
}
