//! Portable ownership of one explicit USB media invitation. Resource-specific
//! adapters project their profile/credential to Request; this owner supervises
//! the original source, exact route and independently proven retirement.
use crate::connection_discovery::{Candidate, Discovery};
use anyhow::{ensure, Result};
use blent_config::connection::{
    management::Resource,
    registration_client::{Client, Factory, Phase, Request},
    routes::RouteId,
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

pub struct MediaRegistration {
    source: Discovery,
    factory: Box<dyn Factory>,
    request: Request,
    candidate: Option<Candidate>,
    client: Option<Box<dyn Client>>,
    route: Option<RouteId>,
    active: bool,
    closing: bool,
}

impl MediaRegistration {
    pub fn new(request: Request, source: Discovery, factory: Box<dyn Factory>) -> Result<Self> {
        ensure!(
            request.resource != Resource::DisplaySession,
            "media registration requires a media resource"
        );
        request.profile.validate(request.resource)?;
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
                "original USB media connection unavailable"
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
            "USB media registration unavailable"
        );
        ensure!(
            self.route.is_none() || self.route == status.route,
            "USB media registration changed route"
        );
        ensure!(
            !self.active || status.phase == Phase::Active,
            "USB media admission withdrawn"
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

impl Drop for MediaRegistration {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
