//! Camera-specific projection to an independent media-registration request.
use anyhow::Result;
use blent_config::{
    camera::{connection::Target, CameraProfile},
    connection::{
        management::{Profile, Resource},
        registration_client::Request,
    },
};

pub fn request(profile: &CameraProfile, token: &str) -> Result<Request> {
    let peer = match profile.target()? {
        Target::Usb(peer) => peer,
        _ => anyhow::bail!("USB camera registration requires an explicit USB peer"),
    };
    crate::media_request::request(
        Target::Usb(peer),
        Resource::Camera,
        Profile::Camera(profile.invitation_profile()?),
        token,
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;
