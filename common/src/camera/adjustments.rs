//! Independent image adjustment values; native adapters own no contract types.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "platform", derive(clap::ValueEnum))]
#[serde(rename_all = "lowercase")]
pub enum ShadowMode {
    Off,
    #[default]
    Auto,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    pub shadows: ShadowMode,
    pub strength: u8,
    pub rotation: u16,
    pub mirror: bool,
}
impl Image {
    pub fn oriented(mut self, sensor: u8) -> anyhow::Result<Self> {
        anyhow::ensure!(sensor <= 3, "Invalid camera sensor orientation");
        self.rotation = (self.rotation + u16::from(sensor) * 90) % 360;
        Ok(self)
    }
    pub fn from_profile(profile: &super::CameraProfile) -> Self {
        Self {
            shadows: profile.shadow_mode,
            strength: profile.shadow_strength,
            rotation: profile.rotation,
            mirror: profile.mirror,
        }
    }
    pub fn write_profile(self, profile: &mut super::CameraProfile) {
        profile.shadow_mode = self.shadows;
        profile.shadow_strength = self.strength;
        profile.rotation = self.rotation;
        profile.mirror = self.mirror;
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub pending: bool,
    pub restart_required: bool,
    pub error: Option<String>,
}
