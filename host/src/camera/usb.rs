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
    listener: TcpListener,
    launch: Launch,
) -> Result<Bridge> {
    crate::media_connection::linux::native(
        profile,
        token,
        crate::camera_registration::request(profile, token)?,
        listener,
        launch,
    )
}
