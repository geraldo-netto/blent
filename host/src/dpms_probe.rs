//! T1013: native Xlib calls run only in an isolated, read-only helper process.
use anyhow::{ensure, Result};
use blent_config::host_activity::DisplayObservation;
use std::{ffi::OsString, io::Write, time::Duration};
mod device;
mod library;
use device::Device;

struct Options {
    interval: Duration,
    samples: u32,
}
impl Options {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self> {
        let interval: u64 = args
            .next()
            .and_then(|v| v.into_string().ok())
            .ok_or_else(|| anyhow::anyhow!("missing DPMS probe interval"))?
            .parse()?;
        ensure!(
            (1..=1_000_000_000).contains(&interval),
            "invalid DPMS probe interval"
        );
        let samples: u32 = args
            .next()
            .and_then(|v| v.into_string().ok())
            .ok_or_else(|| anyhow::anyhow!("missing DPMS probe sample bound"))?
            .parse()?;
        ensure!(
            samples <= 1_000_000 && args.next().is_none(),
            "invalid DPMS probe sample bound or arguments"
        );
        Ok(Self {
            interval: Duration::from_nanos(interval),
            samples,
        })
    }
}
pub fn dispatch(mut args: impl Iterator<Item = OsString>) -> Option<Result<()>> {
    if args.next()?.to_str() != Some("--blent-activity-dpms") {
        return None;
    }
    Some(run(Options::parse(args)))
}
fn run(options: Result<Options>) -> Result<()> {
    let options = options?;
    let device = Device::open()?;
    let mut output = std::io::stdout().lock();
    produce(&device, &mut output, options)
}
fn produce(device: &Device, output: &mut impl Write, options: Options) -> Result<()> {
    let mut count = 0u32;
    loop {
        let sample = device.sample()?;
        publish(output, sample)?;
        count = count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("DPMS sample counter exhausted"))?;
        if !sample.supported || (options.samples != 0 && count >= options.samples) {
            return Ok(());
        }
        std::thread::sleep(options.interval);
    }
}
fn publish(output: &mut impl Write, observation: DisplayObservation) -> Result<()> {
    output.write_all(&observation.encode()?)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests;
