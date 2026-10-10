#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    blent::runtime_owner::run(blent::broker::linux::entry::run(std::env::args_os()))?
}
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("USB broker backend is unavailable on this operating system")
}
