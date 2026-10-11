//! Portable ownership of one explicit USB camera invitation. Snapshot identity
//! is not admission; the original registration must independently become Active.
use crate::connection_discovery::{Candidate, Discovery};
use anyhow::{ensure, Result};
use blent_config::{
    camera::{
        connection::{validate_credential, Target},
        CameraProfile,
    },
    connection::{
        management::{Credential, Profile, Resource},
        registration_client::{Client, Factory, Phase, Request},
        routes::RouteId,
    },
};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Waiting,
    Registering,
    Active,
    Retiring,
    Retired,
}

pub struct CameraRegistration {
    source: Discovery,
    factory: Box<dyn Factory>,
    request: Request,
    candidate: Option<Candidate>,
    client: Option<Box<dyn Client>>,
    route: Option<RouteId>,
    active: bool,
    closing: bool,
}

fn credential(token: &str) -> Result<Credential> {
    validate_credential(token)?;
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&token[index * 2..index * 2 + 2], 16)?;
    }
    Ok(Credential::new(bytes))
}

impl CameraRegistration {
    pub fn new(
        profile: &CameraProfile,
        token: &str,
        source: Discovery,
        factory: Box<dyn Factory>,
    ) -> Result<Self> {
        let peer = match profile.target()? {
            Target::Usb(peer) => peer,
            _ => anyhow::bail!("USB camera registration requires an explicit USB peer"),
        };
        let request = Request {
            peer,
            resource: Resource::Camera,
            credential: credential(token)?,
            profile: Profile::Camera(profile.invitation_profile()?),
        };
        Ok(Self {
            source,
            factory,
            request,
            candidate: None,
            client: None,
            route: None,
            active: false,
            closing: false,
        })
    }

    /// One registration only. A fresh invitation creates a new owner/credential;
    /// failure never quietly redirects this lifetime to a replacement source.
    pub fn poll(&mut self, now: Instant) -> Result<()> {
        let result = self.advance(now);
        if result.is_err() {
            self.close();
        }
        result
    }

    fn advance(&mut self, now: Instant) -> Result<()> {
        let candidates = self.source.poll(now)?;
        if self.closing {
            return Ok(());
        }
        if let Some(candidate) = &self.candidate {
            ensure!(
                self.source.current(candidate),
                "original USB camera connection unavailable"
            );
        } else if let Some(candidate) = candidates
            .into_iter()
            .find(|candidate| candidate.peer() == &self.request.peer)
        {
            self.candidate = Some(candidate);
            self.client = Some(self.factory.start(self.request.clone())?);
        }
        self.observe()
    }

    fn observe(&mut self) -> Result<()> {
        let Some(client) = &self.client else {
            return Ok(());
        };
        let status = client.status();
        ensure!(
            !client.retired() && status.live(),
            "USB camera registration unavailable"
        );
        ensure!(
            self.route.is_none() || self.route == status.route,
            "USB camera registration changed route"
        );
        ensure!(
            !self.active || status.phase == Phase::Active,
            "USB camera admission withdrawn"
        );
        self.route = status.route;
        self.active = status.phase == Phase::Active;
        Ok(())
    }

    pub fn state(&self) -> State {
        if self.closing {
            if self.retired() {
                State::Retired
            } else {
                State::Retiring
            }
        } else if self.active {
            State::Active
        } else if self.client.is_some() {
            State::Registering
        } else {
            State::Waiting
        }
    }

    pub fn close(&mut self) {
        self.closing = true;
        self.active = false;
        self.source.close();
        if let Some(client) = &self.client {
            client.close();
        }
    }

    pub fn retired(&self) -> bool {
        self.closing
            && self.source.retired()
            && self.client.as_ref().is_none_or(|client| client.retired())
    }
}

impl Drop for CameraRegistration {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;
