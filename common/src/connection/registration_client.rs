//! Portable local media-registration client contract. An authenticated native
//! adapter supplies listener capabilities separately; addresses and OS handles
//! never enter this model. One exchange owns one resource, not its connection.
use super::{
    management::{Credential, Profile, Resource},
    pairing::PeerId,
    registration::{Message, Rejection, State},
    routes::RouteId,
};
use anyhow::{ensure, Result};

#[derive(Clone)]
pub struct Request {
    pub peer: PeerId,
    pub resource: Resource,
    pub credential: Credential,
    pub profile: Profile,
}
impl Request {
    fn message(&self) -> Message {
        Message::Register {
            request: 1,
            peer: self.peer.clone(),
            resource: self.resource,
            credential: self.credential.clone(),
            profile: self.profile.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Registering,
    Queued,
    Active,
    Closing,
    Retired,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Rejected(Rejection),
    Protocol,
    Native,
    TimedOut,
    Resource,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub route: Option<RouteId>,
    pub failure: Option<Failure>,
    /// True only when no registration was exposed, a registration was rejected
    /// before ownership, or the exact broker route explicitly retired. Never a
    /// timeout, disconnected IPC socket, failed status or native worker exit.
    pub remote_retired: bool,
}

pub trait Client: Send + Sync {
    fn status(&self) -> Status;
    /// Prompt logical revocation. Actual endpoint/native retirement is separate.
    fn close(&self);
    /// Requires remote_retired AND actual client-owned native worker retirement.
    fn retired(&self) -> bool;
}

/// Bound registration adapter. Native listener capabilities remain outside this
/// contract. Errors prove failure before exposure; uncertain work returns a
/// retained client whose actual retirement must still be observed.
pub trait Factory: Send {
    fn start(&mut self, request: Request) -> Result<Box<dyn Client>>;
}

impl Status {
    /// Shared snapshot invariants, independent of a resource-specific owner.
    pub fn live(&self) -> bool {
        if self.failure.is_some() || self.remote_retired {
            return false;
        }
        match self.phase {
            Phase::Starting | Phase::Registering => self.route.is_none(),
            Phase::Queued | Phase::Active => self.route.is_some_and(|route| {
                (1..=super::wire::MAX_COUNTER).contains(&route.owner)
                    && (1..=super::wire::MAX_COUNTER).contains(&route.revision)
            }),
            _ => false,
        }
    }
}

/// Serialized protocol owner. Popping an outgoing message records possible
/// exposure BEFORE native I/O; cancellation must not assume an interrupted send
/// had no effect. The adapter must continue receiving cleanup proof after close.
/// Request IDs 1/2 belong to this one IPC lifetime and are never reused there.
pub struct Model {
    request: Request,
    status: Status,
    exposed: bool,
    closing: bool,
    cancel_sent: bool,
    observed: Option<State>,
}
impl Model {
    pub fn new(request: Request) -> Result<Self> {
        request.message().encode()?;
        Ok(Self {
            request,
            status: Status {
                phase: Phase::Starting,
                route: None,
                failure: None,
                remote_retired: false,
            },
            exposed: false,
            closing: false,
            cancel_sent: false,
            observed: None,
        })
    }
    pub fn status(&self) -> Status {
        self.status.clone()
    }
    pub fn outgoing(&mut self) -> Option<Message> {
        if self.status.remote_retired {
            return None;
        }
        if !self.exposed {
            self.exposed = true;
            self.status.phase = Phase::Registering;
            return Some(self.request.message());
        }
        self.unregister()
    }
    fn unregister(&mut self) -> Option<Message> {
        if !self.closing || self.cancel_sent {
            return None;
        }
        let route = self.status.route?;
        self.cancel_sent = true;
        Some(Message::Unregister { request: 2, route })
    }
    pub fn close(&mut self) {
        self.closing = true;
        if !self.exposed {
            self.status.remote_retired = true;
            self.status.phase = Phase::Retired;
        } else if !self.status.remote_retired && self.status.failure.is_none() {
            self.status.phase = Phase::Closing;
        }
    }
    pub fn fail(&mut self, failure: Failure) {
        self.close();
        self.status.failure.get_or_insert(failure);
        self.status.phase = Phase::Failed;
    }
    pub fn receive(&mut self, message: Message) -> Result<()> {
        let result = self.apply(message);
        if result.is_err() {
            self.fail(Failure::Protocol);
        }
        result
    }
    fn apply(&mut self, message: Message) -> Result<()> {
        ensure!(
            self.exposed && !self.status.remote_retired,
            "no live registration exchange"
        );
        message.encode()?; // Validate constructed values as well as decoded packets.
        match message {
            Message::Registered { request, route } => self.registered(request, route),
            Message::Status {
                request,
                route,
                state,
            } => self.progress(request, route, state),
            Message::Rejected { request, reason } => self.rejected(request, reason),
            _ => anyhow::bail!("unexpected registration response"),
        }
    }
    fn registered(&mut self, request: u64, route: RouteId) -> Result<()> {
        ensure!(
            request == 1 && self.status.route.is_none(),
            "foreign or duplicate registration"
        );
        self.status.route = Some(route);
        if !self.closing {
            self.status.phase = Phase::Queued;
        }
        Ok(())
    }
    fn progress(&mut self, request: u64, route: RouteId, state: State) -> Result<()> {
        ensure!(
            self.status.route == Some(route),
            "foreign or unknown registration route"
        );
        ensure!(
            request == 1 || (request == 2 && self.cancel_sent),
            "foreign registration request"
        );
        ensure!(
            advance_allowed(self.observed, state),
            "registration state regressed"
        );
        self.observed = Some(state);
        self.phase(state);
        Ok(())
    }
    fn phase(&mut self, state: State) {
        match state {
            State::Retired => {
                self.status.remote_retired = true;
                self.status.phase = Phase::Retired;
            }
            State::Failed => self.fail(Failure::Resource),
            State::Closing => self.close(),
            State::Active if !self.closing => self.status.phase = Phase::Active,
            _ => (), // Queued or late Active cannot undo cancellation/failure.
        }
    }
    fn rejected(&mut self, request: u64, reason: Rejection) -> Result<()> {
        let unowned = request == 1 && self.status.route.is_none();
        ensure!(
            unowned || (request == 2 && self.cancel_sent),
            "foreign registration rejection"
        );
        self.fail(Failure::Rejected(reason));
        if unowned {
            self.status.remote_retired = true;
        }
        Ok(())
    }
}
fn advance_allowed(previous: Option<State>, next: State) -> bool {
    match previous {
        None | Some(State::Queued) => true,
        Some(State::Active) => next != State::Queued,
        Some(State::Closing | State::Failed) => {
            matches!(next, State::Closing | State::Failed | State::Retired)
        }
        Some(State::Retired) => false,
    }
}

#[cfg(test)]
mod tests;
