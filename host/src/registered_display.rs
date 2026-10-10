//! T920: portable ownership of one display's renewable broker registration.
//! The caller exclusively owns the attachment and capacity ticket until stopped
//! and retired. Native session retirement is an additional, independent barrier.
use crate::attachment::{Attachment, Lease, RegisteredRoute};
use anyhow::{ensure, Result};
use blent_config::{
    connection::{
        management::{Credential, Profile, Resource},
        pairing::PeerId,
        registration_client::{Client, Phase, Request, Status},
        routes::{Link, RouteId},
    },
    credentials::Entropy,
};
use std::time::{Duration, Instant};

/// Bound native adapter: owns listener capabilities outside this contract.
/// An error proves startup failed before exposure. If native ownership becomes
/// uncertain, return a client that retains it until actual retirement instead.
pub trait Registration: Send {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Waiting,
    Registering,
    Active,
    Retiring,
    Retired,
}
struct Attempt {
    route: RegisteredRoute,
    client: Box<dyn Client>,
    id: Option<RouteId>,
    admitted: bool,
    closing: bool,
    revoked: Option<Lease>,
}
impl Attempt {
    fn close(&mut self) {
        if !self.closing {
            self.closing = true;
            self.revoked = self.route.revoke();
            self.client.close();
        }
    }
    fn observe(&mut self, status: Status) {
        if !self.route.current() || !self.accepts(&status) {
            self.close();
            return;
        }
        if status.phase == Phase::Active && !self.admitted {
            if self.route.admitted() {
                self.admitted = true;
            } else {
                self.close();
            }
        }
    }
    fn accepts(&mut self, status: &Status) -> bool {
        if self.closing || !live_status(status) {
            return false;
        }
        if self.id.is_some() && self.id != status.route {
            return false;
        }
        self.id = status.route;
        !self.admitted || status.phase == Phase::Active
    }

    fn state(&self) -> State {
        if self.closing {
            State::Retiring
        } else if self.admitted {
            State::Active
        } else {
            State::Registering
        }
    }
}
fn live_status(status: &Status) -> bool {
    use blent_config::connection::wire::MAX_COUNTER;
    if status.failure.is_some() || status.remote_retired {
        return false;
    }
    match status.phase {
        Phase::Starting | Phase::Registering => status.route.is_none(),
        Phase::Queued | Phase::Active => status.route.is_some_and(|route| {
            (1..=MAX_COUNTER).contains(&route.owner) && (1..=MAX_COUNTER).contains(&route.revision)
        }),
        _ => false,
    }
}
pub struct Display {
    peer: PeerId,
    registration: Box<dyn Registration>,
    entropy: Entropy,
    retry: Duration,
    next: Option<Instant>,
    attempt: Option<Attempt>,
    authority: Option<Lease>,
    closing: bool,
}
impl Display {
    pub fn new(
        peer: PeerId,
        attachment: Attachment,
        registration: Box<dyn Registration>,
        entropy: Entropy,
        retry: Duration,
    ) -> Result<Self> {
        ensure!(
            !retry.is_zero() && retry <= Duration::from_secs(60),
            "invalid display retry interval"
        );
        Ok(Self {
            peer,
            authority: Some(attachment.lease()),
            registration,
            entropy,
            retry,
            next: None,
            attempt: None,
            closing: false,
        })
    }
    pub fn state(&self) -> State {
        match &self.attempt {
            Some(attempt) => attempt.state(),
            None if self.closing => State::Retired,
            None => State::Waiting,
        }
    }
    /// Poll only serialized by the owner; native adapters must make status,
    /// close and retired nonblocking. Absolute retry deadlines never slide.
    pub fn poll(&mut self, now: Instant) -> Result<()> {
        self.observe(now);
        if self.closing
            || self.attempt.is_some()
            || self.next.is_some_and(|deadline| now < deadline)
        {
            return Ok(());
        }
        self.next = now.checked_add(self.retry);
        ensure!(self.next.is_some(), "display retry deadline exhausted");
        self.start()
    }
    fn observe(&mut self, now: Instant) {
        let Some(attempt) = &mut self.attempt else {
            return;
        };
        attempt.observe(attempt.client.status());
        if !attempt.closing {
            return;
        }
        if attempt.revoked.is_none() {
            self.closing = true;
        }
        if attempt.client.retired() {
            self.authority = self.attempt.take().and_then(|attempt| attempt.revoked);
            self.next = now.checked_add(self.retry);
        }
    }
    fn start(&mut self) -> Result<()> {
        let mut bytes = [0; 32];
        (self.entropy)(&mut bytes)?;
        let credential = Credential::new(bytes);
        let route = match self
            .authority
            .as_ref()
            .expect("idle display authority")
            .register_route(self.peer.clone(), credential.clone(), Link::Usb)
        {
            Ok(route) => route,
            Err(error) => {
                self.closing = true;
                return Err(error);
            }
        };
        let client = match self.registration.start(Request {
            peer: self.peer.clone(),
            resource: Resource::DisplaySession,
            credential,
            profile: Profile::Display,
        }) {
            Ok(client) => client,
            Err(error) => {
                self.authority = route.revoke();
                self.closing = self.authority.is_none();
                return Err(error);
            }
        };
        self.authority = None;
        self.attempt = Some(Attempt {
            route,
            client,
            id: None,
            admitted: false,
            closing: false,
            revoked: None,
        });
        Ok(())
    }
    /// Whole-display cancellation. Unlike automatic route renewal, this clears
    /// desired presence; repeated close/drop cannot alter a later attachment.
    pub fn close(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        if let Some(attempt) = &mut self.attempt {
            attempt.close();
            if let Some(revoked) = &attempt.revoked {
                revoked.detach_registered();
            }
        }
        if let Some(revoked) = &self.authority {
            revoked.detach_registered();
        }
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        self.close();
    }
}
#[cfg(test)]
mod tests;
