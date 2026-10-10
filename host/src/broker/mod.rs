//! Host connection broker adapters. Portable registration contracts live in
//! blent_config::connection; OS socket/descriptor APIs stay inside backends.
#[cfg(target_os = "linux")]
pub mod linux;
