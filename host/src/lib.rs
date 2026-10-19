//! Reusable host resource sharing, independent of display capture.
#[cfg(target_os = "linux")]
pub mod activity;
pub mod activity_follow;
#[cfg(target_os = "linux")]
pub mod audio;
pub mod audio_control;
#[cfg(target_os = "linux")]
pub mod camera;
pub mod camera_control;
pub mod camera_registration;
pub mod capture_contract;
mod capture_retirement;
pub mod config_jobs;
mod control_outcome;
pub mod control_workers;
#[cfg(target_os = "linux")]
pub mod dpms_probe;
pub mod owned_task;
pub mod persistence_port;
mod session_adapters;
pub mod session_network;
pub mod session_state;
mod session_supervision;

// Shared wire/session state is built for every host target.
pub mod adb_inventory;
pub mod attachment;
pub mod config;
pub mod connection_discovery;
pub mod display_connections;
pub mod display_slots;
pub mod display_quiescence;
pub mod ffmpeg_args;
pub mod latency;
pub mod media;
#[cfg(any(test, all(target_os = "linux", feature = "accessory-transport")))]
pub(crate) mod media_connection;
pub mod media_registration;
pub mod media_request;
pub mod media_storage;
pub mod pipe_encoder;
pub mod raw_transfer;
pub mod registered_display;
pub mod registered_session;
#[path = "selection_types.rs"]
pub mod selection;
pub mod session_work;
pub mod tray_state;
pub mod usb;
pub mod video_queue;
#[cfg(windows)]
pub mod windows_tray;

#[cfg(test)]
mod test_logging;

pub mod input;
pub mod stream;

// Linux display/input adapters are never linked into other host targets.
#[cfg(test)]
#[allow(dead_code)] // Shared probe also supports the binary packetizer benchmarks.
mod allocation_probe;
#[cfg(target_os = "linux")]
pub mod kscreen;
#[cfg(target_os = "linux")]
pub mod kwin;
#[cfg(target_os = "linux")]
pub mod osk;
#[cfg(test)]
mod poll_probe;
#[cfg(target_os = "linux")]
pub mod vdisplay;

#[path = "session_core.rs"]
pub mod session;

mod annex_b;
pub mod annex_scan;
pub mod encoder_probe;
mod framed_annex_b;
#[path = "capture/probe_format.rs"]
pub mod probe_format;

#[path = "monitor/launch_policy.rs"]
mod launch_policy;
pub mod transport;
pub mod wifi;

mod command_output;
pub mod update;

pub mod encoder_worker_contract;
pub mod runtime_owner;

#[cfg(feature = "accessory-transport")]
pub mod secure_connection;

#[cfg(feature = "accessory-transport")]
pub mod accessory;

#[cfg(feature = "accessory-transport")]
pub mod broker;

#[cfg(test)]
mod display_test_support;

pub mod native_scope;

pub mod work_admission;

pub mod activity_view;

pub mod display_work;

pub mod current_work;

pub mod auxiliary_work;

#[cfg(target_os = "linux")]
pub mod auxiliary_source;
