//! Typed camera adapter. Shared supervision and retained cleanup belong to the
//! portable media owner; Linux descriptor setup stays in its native boundary.
use crate::broker::linux::client::Launch;
use anyhow::Result;
use blent_config::camera::CameraProfile;
use std::net::TcpListener;

pub(crate) type Bridge = crate::media_connection::Bridge<CameraProfile>;
pub(crate) fn native(
    profile: &CameraProfile,
    token: &str,
    create_listener: impl FnOnce() -> std::io::Result<TcpListener>,
    launch: Launch,
    report: &crate::camera_control::Report,
) -> Result<Bridge> {
    let request = crate::camera_registration::request(profile, token)?;
    let original = report.reserve_native(crate::camera_control::NativeResource::Connection)?;
    let listener = create_listener()?;
    let bridge = crate::media_connection::linux::native(profile, token, request, listener, launch)?;
    original.bind(bridge.completion());
    Ok(bridge)
}

#[cfg(test)]
mod retirement_tests;
