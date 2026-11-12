//! Narrow native camera pipeline entry points for the opt-in measurement tool.
//! Reuse the production owners; do not recompile their modules under a new parent.
pub use super::bridge::{Bridge, Target};
pub use super::decoder::connection;
pub use super::outputs::Frames;
pub use crate::adb_command::broadcast_accepted;
