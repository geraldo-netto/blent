//! Linux V4L2 setup on one caller-owned original file capability.
//! This adapter owns its admitted descriptor through synchronous ioctls. Call it from
//! an owned blocking worker; only actual completion hands that same capability to a writer.
use anyhow::{ensure, Result};
use blent_config::camera::CameraProfile;
use std::{fs::File, os::fd::AsFd};

mod abi;
mod native;

/// Immutable, validated image/rate snapshot, independent of native storage.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    width: u32,
    height: u32,
    fps: u32,
    bytes: u32,
}
impl Spec {
    pub fn from_profile(profile: &CameraProfile) -> Result<Self> {
        profile.validate()?;
        Ok(Self {
            width: profile.width,
            height: profile.height,
            fps: profile.fps,
            bytes: u32::try_from(profile.frame_bytes()?)?,
        })
    }
    pub fn dimensions(self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn fps(self) -> u32 {
        self.fps
    }
    pub fn frame_bytes(self) -> u32 {
        self.bytes
    }
}

/// Configure the same already-owned file description that will carry writes.
/// No path lookup, descriptor creation or native task occurs here. The descriptor
/// moves into ConfiguredOutput on success and closes after synchronous failure.
pub fn configure_blocking(output: File, spec: Spec) -> Result<ConfiguredOutput> {
    configure_with(&native::Ioctl, output, spec)
}

/// One configured original capability and its immutable frame contract.
/// Dropping it closes this owned descriptor; other explicit original references
/// retain their own kernel lifetime. Prepare before admitting a native writer.
pub struct ConfiguredOutput {
    output: File,
    spec: Spec,
}
impl ConfiguredOutput {
    pub fn into_parts(self) -> (File, Spec) {
        (self.output, self.spec)
    }
}

fn configure_with(
    control: &impl native::Controls,
    output: File,
    spec: Spec,
) -> Result<ConfiguredOutput> {
    let mut format = abi::Format::new(spec);
    control.set_format(output.as_fd(), &mut format)?;
    format.validate(spec)?;
    configure_rate(control, &output, spec.fps)?;
    Ok(ConfiguredOutput { output, spec })
}

fn configure_rate(control: &impl native::Controls, output: &File, fps: u32) -> Result<()> {
    let mut current = abi::Parameters::new(0);
    control.get_rate(output.as_fd(), &mut current)?;
    if current.matches(fps)? {
        return Ok(());
    }
    ensure!(
        current.can_set(),
        "camera output cannot set the requested rate"
    );
    let mut requested = abi::Parameters::new(fps);
    control.set_rate(output.as_fd(), &mut requested)?;
    ensure!(
        requested.matches(fps)?,
        "camera output adjusted the requested rate"
    );
    Ok(())
}

#[cfg(test)]
mod tests;

/// Test-only native configuration port. It uses the same complete readback
/// validation as the real adapter; it cannot construct unchecked readiness.
#[cfg(test)]
pub(super) fn configure_fixture(output: File, spec: Spec) -> Result<ConfiguredOutput> {
    configure_with(&tests::Control::default(), output, spec)
}
