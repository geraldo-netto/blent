//! Validated projection to the independent registration contract. No discovery,
//! native descriptor, process or device work occurs in this boundary adapter.
use anyhow::{ensure, Result};
use blent_config::connection::{
    management::{Credential, Profile, Resource},
    media::{valid_credential, Target},
    registration_client::Request,
};

pub fn credential(token: &str) -> Result<Credential> {
    ensure!(valid_credential(token), "invalid media credential");
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&token[index * 2..index * 2 + 2], 16)?;
    }
    Ok(Credential::new(bytes))
}

pub fn request(
    target: Target<'_>,
    resource: Resource,
    profile: Profile,
    token: &str,
) -> Result<Request> {
    let peer = match target {
        Target::Usb(peer) => peer,
        _ => anyhow::bail!("media registration requires an explicit USB peer"),
    };
    ensure!(
        resource != Resource::DisplaySession,
        "media registration requires a media resource"
    );
    profile.validate(resource)?;
    Ok(Request {
        peer,
        resource,
        credential: credential(token)?,
        profile,
    })
}

/// The native adapter accepts one validated immutable request. A separate
/// bootstrap token must describe its exact credential before native setup.
pub(crate) fn validate_original(request: &Request, token: &str) -> Result<()> {
    ensure!(
        request.resource != Resource::DisplaySession,
        "media registration requires a media resource"
    );
    request.profile.validate(request.resource)?;
    let credential = credential(token)?;
    let difference = credential
        .as_bytes()
        .iter()
        .zip(request.credential.as_bytes())
        .fold(0u8, |difference, (first, second)| {
            difference | (first ^ second)
        });
    ensure!(
        difference == 0,
        "media bootstrap differs from original registration credential"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
