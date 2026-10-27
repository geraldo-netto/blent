//! T539 independent camera profile; never changes display-stream settings.
use anyhow::{ensure, Result};
#[cfg(feature = "platform")]
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub mod connection;
mod preview;
pub mod stop;
pub use preview::CameraPreview;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum Lens {
    #[default]
    Front,
    Rear,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    #[serde(flatten)]
    pub options: CameraProfile,
}

#[cfg_attr(feature = "platform", derive(Args))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraProfile {
    /// Lens selected by the desktop; Android still requires camera permission.
    #[cfg_attr(feature = "platform", arg(long, value_enum, default_value = "front"))]
    pub lens: Lens,
    /// Continue capture when the tablet app is in the background or locked.
    #[cfg_attr(feature = "platform", arg(long))]
    pub background: bool,
    /// ADB serial; required when more than one device is connected.
    #[cfg_attr(feature = "platform", arg(long))]
    pub serial: Option<String>,
    /// Explicit direct-USB tablet certificate fingerprint; never selects ADB.
    #[cfg_attr(feature = "platform", arg(long, conflicts_with = "serial"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb_peer: Option<String>,
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 1280))]
    pub width: u32,
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 720))]
    pub height: u32,
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 30))]
    pub fps: u32,
    /// Camera H.264 target bitrate in kbit/s; independent of display bitrate.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 3000))]
    pub bitrate: u32,
    /// Lower the camera target under congestion, then recover slowly.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = true, action = clap::ArgAction::Set))]
    pub adaptive_bitrate: bool,
    /// Adaptive floor in kbit/s, capped by the requested bitrate.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 1000))]
    pub min_bitrate: u32,
    /// Extra encoder-queue age and transport budget; not glass-to-glass latency.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 150))]
    pub freshness_ms: u32,
    /// Additional clockwise rotation after Android sensor/display correction.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 0))]
    pub rotation: u16,
    /// Mirror exported camera video horizontally.
    #[cfg_attr(feature = "platform", arg(long))]
    pub mirror: bool,
}

/// Portable UI/backend contract. Only the native adapter owns OS devices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CameraState {
    Stopped,
    Starting,
    /// Manual intent retained while host availability prevents admission.
    Paused,
    Waiting,
    Streaming,
    Stopping,
    Failed(String),
}

impl CameraState {
    /// A retained manual intent keeps Stop available, including while paused.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Stopped | Self::Failed(_))
    }
}

pub type BackendResult = Result<()>;

pub trait CameraBackend {
    fn close(self: Box<Self>) -> crate::retirement::Fence;
    fn start(&self, options: CameraProfile) -> Result<()>;
    fn stop(&self);
    fn state(&self) -> CameraState;
    fn preview(&self) -> Option<Arc<CameraPreview>>;
}

impl Default for CameraProfile {
    fn default() -> Self {
        Self {
            lens: Lens::Front,
            background: false,
            serial: None,
            usb_peer: None,
            width: 1280,
            height: 720,
            fps: 30,
            bitrate: 3000,
            freshness_ms: 150,
            adaptive_bitrate: true,
            min_bitrate: 1000,
            mirror: false,
            rotation: 0,
        }
    }
}

impl CameraProfile {
    pub fn validate(&self) -> Result<()> {
        self.target()?;
        self.validate_freshness()?;
        ensure!(
            matches!(self.rotation, 0 | 90 | 180 | 270),
            "camera rotation must be 0, 90, 180 or 270 degrees"
        );
        self.validate_dimensions()?;
        ensure!((5..=30).contains(&self.fps), "camera FPS must be 5–30");
        ensure!(
            (256..=20000).contains(&self.bitrate),
            "camera bitrate must be 256–20000 kbit/s"
        );
        Ok(())
    }

    fn validate_freshness(&self) -> Result<()> {
        ensure!(
            (256..=20000).contains(&self.min_bitrate),
            "camera minimum bitrate must be 256–20000 kbit/s"
        );
        ensure!(
            (50..=2000).contains(&self.freshness_ms),
            "camera freshness must be 50–2000 ms"
        );
        Ok(())
    }

    fn validate_dimensions(&self) -> Result<()> {
        ensure!(
            (160..=1920).contains(&self.width),
            "camera width must be 160–1920"
        );
        ensure!(
            (120..=1080).contains(&self.height),
            "camera height must be 120–1080"
        );
        ensure!(
            self.width.is_multiple_of(2) && self.height.is_multiple_of(2),
            "camera dimensions must be even"
        );
        Ok(())
    }

    /// Planar YUV420 byte size for supported, even dimensions. Unchecked public
    /// profiles must be rejected before any allocation; unrelated settings do
    /// not affect the layout. Valid bounds also prove the u64 product cannot wrap.
    pub fn frame_bytes(&self) -> Result<usize> {
        self.validate_dimensions()?;
        let bytes = u64::from(self.width) * u64::from(self.height) * 3 / 2;
        Ok(usize::try_from(bytes)?)
    }
}

// Diagnostic CLI options belong to the native adapter, not the saved profile.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::CameraOptions;
#[cfg(not(target_os = "linux"))]
pub type CameraOptions = CameraProfile;

#[cfg(all(test, feature = "platform"))]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn options() -> CameraOptions {
        let Some(Commands::Cameras(options)) = Cli::parse_from(["blent", "cameras"]).command else {
            panic!()
        };
        options
    }

    #[test]
    fn t938_unchecked_dimensions_never_panic_or_wrap_frame_size() {
        let profile = CameraProfile {
            width: u32::MAX,
            height: u32::MAX,
            ..Default::default()
        };
        let size = std::panic::catch_unwind(|| profile.frame_bytes());
        assert!(size.is_ok(), "T938 unchecked public dimensions panicked");
        assert!(
            size.unwrap().is_err(),
            "T938 unsupported frame size accepted"
        );
    }

    #[test]
    fn t938_dimension_boundaries_and_supported_sizes_are_exact() {
        let values = [
            0,
            1,
            119,
            120,
            121,
            159,
            160,
            161,
            480,
            720,
            1080,
            1081,
            1920,
            1921,
            u32::MAX,
        ];
        for width in values {
            for height in values {
                let profile = CameraProfile {
                    width,
                    height,
                    ..Default::default()
                };
                let size = profile.frame_bytes();
                let supported = (160..=1920).contains(&width)
                    && (120..=1080).contains(&height)
                    && width.is_multiple_of(2)
                    && height.is_multiple_of(2);
                assert_eq!(size.is_ok(), supported, "T938 {width}x{height}");
                if supported {
                    assert_eq!(
                        size.unwrap() as u64,
                        u64::from(width) * u64::from(height) * 3 / 2
                    );
                }
            }
        }
    }

    #[test]
    fn t618_adaptive_rate_bounds_and_fixed_override() {
        for value in [0, 255, 256, 1000, 20000, 20001, u32::MAX] {
            let mut candidate = options();
            candidate.min_bitrate = value;
            assert_eq!(candidate.validate().is_ok(), (256..=20000).contains(&value));
        }
        let cli = Cli::parse_from([
            "blent",
            "cameras",
            "--adaptive-bitrate",
            "false",
            "--min-bitrate",
            "256",
        ]);
        let Some(Commands::Cameras(profile)) = cli.command else {
            panic!()
        };
        assert!(!profile.adaptive_bitrate);
        assert_eq!(profile.min_bitrate, 256);
    }

    #[test]
    fn t616_freshness_budget_is_bounded() {
        for value in [0, 49, 50, 150, 2000, 2001, u32::MAX] {
            let mut candidate = options();
            candidate.freshness_ms = value;
            assert_eq!(candidate.validate().is_ok(), (50..=2000).contains(&value));
        }
    }

    #[test]
    fn t539_profile_limits_and_bounded_invalid_inputs() {
        let defaults = options();
        for rotation in [0, 1, 89, 90, 180, 270, 360, u16::MAX] {
            let mut candidate = defaults.clone();
            candidate.rotation = rotation;
            assert_eq!(
                candidate.validate().is_ok(),
                [0, 90, 180, 270].contains(&rotation)
            );
        }
        defaults.validate().unwrap();
        assert_eq!(defaults.frame_bytes().unwrap(), 1280 * 720 * 3 / 2);
        for value in [0, 1, 159, 160, 161, 720, 1080, 1280, 1920, 1921, u32::MAX] {
            let mut candidate = defaults.clone();
            candidate.width = value;
            assert_eq!(
                candidate.validate().is_ok(),
                (160..=1920).contains(&value) && value.is_multiple_of(2)
            );
            candidate = defaults.clone();
            candidate.height = value;
            assert_eq!(
                candidate.validate().is_ok(),
                (120..=1080).contains(&value) && value.is_multiple_of(2)
            );
        }
        for value in 0..64 {
            let mut candidate = defaults.clone();
            candidate.fps = value;
            assert_eq!(candidate.validate().is_ok(), (5..=30).contains(&value));
        }
        for value in [0, 255, 256, 3000, 20000, 20001, u32::MAX] {
            let mut candidate = defaults.clone();
            candidate.bitrate = value;
            assert_eq!(candidate.validate().is_ok(), (256..=20000).contains(&value));
        }
        #[cfg(target_os = "linux")]
        {
            let mut same = defaults;
            same.rear_device = same.front_device.clone();
            assert!(same.validate().is_err());
        }
    }
}

#[cfg(test)]
mod connection_tests;
