//! Selected USB discovery intent. A broker snapshot never admits media by itself;
//! the registered-display owner still requires the exact Active route.
use anyhow::{ensure, Result};
use blent_config::connection::{
    control::State,
    control_client::{Client, Phase, Status},
    control_view::{Lifetime, View},
    pairing::{PeerId, Ticket},
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Prompt startup only. Implementations retain native work independently and
/// expose its actual completion through Client::retired.
pub trait Factory: Send {
    fn start(&mut self) -> Result<Box<dyn Client>>;
}

#[derive(Clone)]
pub struct Candidate {
    origin: Lifetime,
    connection: u64,
    ticket: Ticket,
}
impl Candidate {
    pub fn peer(&self) -> &PeerId {
        &self.ticket.peer
    }
    pub fn connection(&self) -> u64 {
        self.connection
    }
    /// Persisted identity and counters can repeat after broker replacement.
    pub fn same_connection(&self, other: &Self) -> bool {
        self.origin.same_as(&other.origin)
            && self.connection == other.connection
            && self.ticket == other.ticket
    }
    fn current_in(&self, view: &View) -> bool {
        self.origin.same_as(&view.lifetime())
            && view
                .snapshot()
                .links()
                .get(&self.connection)
                .is_some_and(|link| {
                    link.state == State::Authenticated && link.ticket.as_ref() == Some(&self.ticket)
                })
    }
}

fn candidates(view: &View) -> Vec<Candidate> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for link in view
        .snapshot()
        .links()
        .values()
        .filter(|link| link.state == State::Authenticated)
    {
        if let Some(ticket) = &link.ticket {
            *counts.entry(ticket.peer.as_str()).or_default() += 1;
        }
    }
    view.snapshot()
        .links()
        .iter()
        .rev()
        .filter_map(|(&connection, link)| {
            let ticket = link.ticket.as_ref()?;
            (link.state == State::Authenticated && counts.get(ticket.peer.as_str()) == Some(&1))
                .then(|| Candidate {
                    origin: view.lifetime(),
                    connection,
                    ticket: ticket.clone(),
                })
        })
        .collect()
}

/// One retained control client. Failed/closing workers cannot be replaced until
/// they actually retire; retry deadlines are absolute and never slide on polls.
pub struct Discovery {
    factory: Box<dyn Factory>,
    client: Option<Box<dyn Client>>,
    retry: Duration,
    next: Option<Instant>,
    closed: bool,
    last: Option<Status>,
}
impl Discovery {
    pub fn new(factory: Box<dyn Factory>, retry: Duration, now: Instant) -> Result<Self> {
        ensure!(
            !retry.is_zero() && retry <= Duration::from_secs(60),
            "invalid USB discovery retry"
        );
        Ok(Self {
            factory,
            client: None,
            retry,
            next: Some(now),
            closed: false,
            last: None,
        })
    }
    pub fn poll(&mut self, now: Instant) -> Result<Vec<Candidate>> {
        if self.closed {
            return Ok(Vec::new());
        }
        if self.client.is_none() && self.next.is_some_and(|deadline| now >= deadline) {
            self.next = None;
            match self.factory.start() {
                Ok(client) => self.client = Some(client),
                Err(error) => {
                    self.next = now.checked_add(self.retry);
                    self.last = Some(terminal(Status {
                        phase: Phase::Failed,
                        view: None,
                        message: Some(format!("{error:#}")),
                        outcome: None,
                    }));
                    return Err(error);
                }
            }
        }
        Ok(self.observe(now))
    }
    fn observe(&mut self, now: Instant) -> Vec<Candidate> {
        let Some(client) = &self.client else {
            return Vec::new();
        };
        let status = client.status();
        if matches!(status.phase, Phase::Closing | Phase::Closed | Phase::Failed)
            || client.retired()
        {
            client.close();
            if client.retired() {
                self.last = Some(terminal(status));
                self.client = None;
                self.next = now.checked_add(self.retry);
            }
            return Vec::new();
        }
        if !matches!(status.phase, Phase::Ready | Phase::Busy) {
            return Vec::new();
        }
        status.view.as_ref().map(candidates).unwrap_or_default()
    }
    pub fn status(&self) -> Option<Status> {
        self.client
            .as_ref()
            .map(|client| client.status())
            .or_else(|| self.last.clone())
    }
    /// Existing registrations retain their exact link when another selected
    /// location proves the same certificate. Ambiguity blocks new admission.
    pub fn current(&self, candidate: &Candidate) -> bool {
        if self.closed {
            return false;
        }
        let Some(client) = &self.client else {
            return false;
        };
        let status = client.status();
        !client.retired()
            && matches!(status.phase, Phase::Ready | Phase::Busy)
            && status
                .view
                .as_ref()
                .is_some_and(|view| candidate.current_in(view))
    }
    pub fn close(&mut self) {
        self.closed = true;
        self.next = None;
        if let Some(client) = &self.client {
            client.close();
        }
    }
    pub fn retired(&self) -> bool {
        self.closed && self.client.as_ref().is_none_or(|client| client.retired())
    }
}
fn terminal(mut status: Status) -> Status {
    if !matches!(status.phase, Phase::Closing | Phase::Closed | Phase::Failed) {
        status.phase = Phase::Closed;
    }
    status.view = None;
    status.outcome = None;
    status.message = status
        .message
        .map(|message| message.chars().take(512).collect());
    status
}
impl Drop for Discovery {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
