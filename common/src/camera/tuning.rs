//! Portable camera controls and capability schema. Camera2 objects stay in Android.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Metering {
    #[default]
    Device,
    Center,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Noise {
    #[default]
    Device,
    Off,
    Fast,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Flicker {
    #[default]
    Device,
    Auto,
    Hz50,
    Hz60,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(clap::Args))]
#[serde(default, deny_unknown_fields)]
pub struct Sensor {
    /// Exposure compensation in hundredths of an EV; rounded to camera steps.
    #[cfg_attr(feature = "platform", arg(long, default_value_t = 0))]
    pub exposure: i16,
    #[cfg_attr(feature = "platform", arg(long, value_enum, default_value = "device"))]
    pub metering: Metering,
    #[cfg_attr(feature = "platform", arg(long, value_enum, default_value = "device"))]
    pub noise: Noise,
    #[cfg_attr(feature = "platform", arg(long, value_enum, default_value = "device"))]
    pub flicker: Flicker,
}
impl Sensor {
    pub fn validate(self) -> Result<()> {
        ensure!(
            (-300..=300).contains(&self.exposure),
            "Exposure must be -3 to +3 EV"
        );
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    pub ceiling: u16,
    pub floor: u16,
    pub adaptive: bool,
}
impl Encoding {
    pub fn from_profile(profile: &super::CameraProfile) -> Self {
        Self {
            ceiling: profile.bitrate as u16,
            floor: profile.min_bitrate as u16,
            adaptive: profile.adaptive_bitrate,
        }
    }
    pub fn write_profile(self, profile: &mut super::CameraProfile) {
        profile.bitrate = self.ceiling.into();
        profile.min_bitrate = self.floor.into();
        profile.adaptive_bitrate = self.adaptive;
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub revision: u64,
    pub sensor: Sensor,
    pub encoding: Encoding,
}
impl Command {
    /// Exactly 18 bytes, following each connection-local packet ACK.
    pub fn encode(self) -> [u8; 18] {
        let mut out = [0; 18];
        out[..8].copy_from_slice(&self.revision.to_be_bytes());
        out[8..10].copy_from_slice(&self.sensor.exposure.to_be_bytes());
        out[10] = self.sensor.metering as u8;
        out[11] = self.sensor.noise as u8;
        out[12] = self.sensor.flicker as u8;
        out[13..15].copy_from_slice(&self.encoding.ceiling.to_be_bytes());
        out[15..17].copy_from_slice(&self.encoding.floor.to_be_bytes());
        out[17] = u8::from(self.encoding.adaptive);
        out
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    pub minimum: i32,
    pub maximum: i32,
    pub numerator: u32,
    pub denominator: u32,
    pub center: bool,
    pub noise_mask: u8,
    pub flicker_mask: u8,
}
impl Capabilities {
    pub fn decode(bytes: [u8; 20]) -> Result<Self> {
        ensure!(
            bytes[16] <= 1 && bytes[19] == 0,
            "Invalid camera capability flags"
        );
        let value = Self {
            minimum: i32::from_be_bytes(bytes[..4].try_into()?),
            maximum: i32::from_be_bytes(bytes[4..8].try_into()?),
            numerator: u32::from_be_bytes(bytes[8..12].try_into()?),
            denominator: u32::from_be_bytes(bytes[12..16].try_into()?),
            center: bytes[16] != 0,
            noise_mask: bytes[17],
            flicker_mask: bytes[18],
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(self) -> Result<()> {
        ensure!(
            (-1000..=0).contains(&self.minimum) && (0..=1000).contains(&self.maximum),
            "Invalid camera exposure range"
        );
        ensure!(
            self.numerator <= 1_000_000 && (1..=1_000_000).contains(&self.denominator),
            "Invalid camera exposure step"
        );
        ensure!(
            self.noise_mask & 1 == 1 && self.noise_mask <= 7,
            "Invalid noise capabilities"
        );
        ensure!(
            self.flicker_mask & 1 == 1 && self.flicker_mask <= 15,
            "Invalid flicker capabilities"
        );
        Ok(())
    }
    pub fn exposure_bounds(self) -> (f64, f64, f64) {
        let step = f64::from(self.numerator) / f64::from(self.denominator);
        (
            (f64::from(self.minimum) * step).max(-3.0),
            (f64::from(self.maximum) * step).min(3.0),
            step,
        )
    }
    pub fn accepts(self, sensor: Sensor) -> Result<()> {
        sensor.validate()?;
        let (minimum, maximum, _) = self.exposure_bounds();
        let ev = f64::from(sensor.exposure) / 100.0;
        ensure!(
            ev >= minimum - 0.001 && ev <= maximum + 0.001,
            "Exposure outside camera range"
        );
        ensure!(
            sensor.metering != Metering::Center || self.center,
            "Center metering unavailable"
        );
        ensure!(
            self.noise_mask & (1 << sensor.noise as u8) != 0,
            "Noise mode unavailable"
        );
        ensure!(
            self.flicker_mask & (1 << sensor.flicker as u8) != 0,
            "Flicker mode unavailable"
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests;
