//! Current probe admission, independent of ADB/process/socket implementations.
use anyhow::{ensure, Result};
use blent_config::camera::{connection::Transports, CameraProfile};

const TRANSPORTS: Transports = Transports {
    adb: true,
    usb: false,
};

pub(super) fn validate(profile: &CameraProfile, seconds: u64, stall_ms: u64) -> Result<()> {
    profile.validate()?;
    ensure!(
        TRANSPORTS.supports(&profile.target()?),
        "camera-feedback-probe supports ADB targets only"
    );
    ensure!(
        (1..=30).contains(&seconds),
        "probe duration must be 1–30 seconds"
    );
    ensure!(stall_ms <= 1000, "stall must be ≤1000 ms");
    Ok(())
}
