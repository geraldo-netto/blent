//! Host connection broker adapters. Portable registration contracts live in
//! blent_config::connection; OS socket/descriptor APIs stay inside backends.
#[cfg(target_os = "linux")]
pub mod linux;

pub mod transport;

pub mod session;

pub mod connection;
pub mod pump;
pub mod window;

pub mod authority_worker;
pub mod bridges;
pub mod control;
pub mod link;
