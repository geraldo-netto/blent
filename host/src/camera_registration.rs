//! Camera-specific projection to an independent media-registration request.
use anyhow::Result;
use blent_config::{
    camera::{
        connection::{validate_credential, Target},
        CameraProfile,
    },
    connection::{
        management::{Credential, Profile, Resource},
        registration_client::Request,
    },
};

fn credential(token: &str) -> Result<Credential> {
    validate_credential(token)?;
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&token[index * 2..index * 2 + 2], 16)?;
    }
    Ok(Credential::new(bytes))
}

pub fn request(profile: &CameraProfile, token: &str) -> Result<Request> {
    let peer = match profile.target()? {
        Target::Usb(peer) => peer,
        _ => anyhow::bail!("USB camera registration requires an explicit USB peer"),
    };
    Ok(Request {
        peer,
        resource: Resource::Camera,
        credential: credential(token)?,
        profile: Profile::Camera(profile.invitation_profile()?),
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;
