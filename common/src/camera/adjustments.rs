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
