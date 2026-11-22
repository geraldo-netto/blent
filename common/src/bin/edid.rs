//! Development file adapter for the shared EDID encoder.
use anyhow::{Context, Result};
use blent_config::{display, edid::Mode};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blent-edid", about = "Generate one Blent EDID block")]
struct Args {
    #[arg(default_value_t = 2960)]
    width: u32,
    #[arg(default_value_t = 1848)]
    height: u32,
    #[arg(default_value_t = 60)]
    refresh: u32,
    #[arg(default_value = "s9ultra.bin")]
    output: PathBuf,
    #[arg(default_value_t = display::DEFAULT_WIDTH_MM)]
    width_mm: u32,
    #[arg(default_value_t = display::DEFAULT_HEIGHT_MM)]
    height_mm: u32,
    #[arg(long, default_value_t = 0)]
    instance: u32,
    #[arg(long, default_value = "Blent")]
    name: String,
}
impl Args {
    fn mode(&self) -> Mode {
        Mode {
            width: self.width,
            height: self.height,
            refresh: self.refresh,
            width_mm: self.width_mm,
            height_mm: self.height_mm,
            instance: self.instance,
        }
    }
}
fn run(args: &Args) -> Result<()> {
    // Reject the complete request before opening or replacing an output file.
    let block = args.mode().encode(&args.name)?;
    std::fs::write(&args.output, block).context("write EDID output")?;
    println!("EDID generated: {} (128 bytes)", args.output.display());
    Ok(())
}
fn main() -> Result<()> {
    run(&Args::parse())
}
