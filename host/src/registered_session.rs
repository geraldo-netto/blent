//! One capacity assignment joins independently owned registration and session work.
//! Native listener/runtime bindings stay outside the lifecycle contracts.
use crate::{
    attachment::Attachment,
    connection_discovery::Candidate,
    display_slots::{Reservations, Ticket},
    registered_display::{Display, State},
};
use anyhow::{ensure, Result};
use blent_config::{
    connection::registration_client::Factory as Registration,
    credentials::Entropy,
    tablets::{Identity, TabletSession},
};
use std::time::{Duration, Instant};

/// Prompt, nonblocking native session port. A reusable primary binding owns no
/// per-assignment capture runtime. Owned extras must prove actual retirement.
pub trait Work: Send {
    fn alive(&self) -> bool;
    fn close(&mut self);
    fn retired(&self) -> bool;
}

pub struct Request {
    candidate: Candidate,
    ticket: Ticket,
    ports: (u16, u16),
    retry: Duration,
}
impl Request {
    /// Validate before starting native work. One Slots registry belongs to the
    /// daemon; its opaque tickets must not be imported from another registry.
    pub fn new(
        slots: &dyn Reservations,
        candidate: Candidate,
        ticket: Ticket,
        ports: (u16, u16),
        retry: Duration,
    ) -> Result<Self> {
        ensure!(
            slots.open_peer(ticket, candidate.peer()),
            "USB display does not own an open capacity ticket"
        );
        ensure!(
            ports.0 != 0 && ports.1 != 0 && ports.0 != ports.1,
            "invalid USB display ports"
        );
        ensure!(
            !retry.is_zero() && retry <= Duration::from_secs(60),
            "invalid USB display retry"
        );
        Ok(Self {
            candidate,
            ticket,
            ports,
            retry,
        })
    }
    /// Composition-owned bindings, separate from the portable Work port.
    /// Revalidate after native startup. Cancellation returns a closing owner
    /// retaining cleanup proof and capabilities, without touching a replacement.
    pub fn bind(self, slots: &dyn Reservations, bindings: Bindings) -> Session {
        let current = self.current(slots);
        let mut registration = Some(bindings.registration);
        let display = current.then(|| {
            Display::new(
                self.candidate.peer().clone(),
                bindings.attachment,
                registration.take().expect("unexposed registration binding"),
                bindings.entropy,
                self.retry,
            )
            .expect("validated USB display request")
        });
        let mut session = Session {
            request: self,
            display,
            _unregistered: registration,
            work: bindings.work,
            closing: false,
        };
        if !current {
            session.close();
        }
        session
    }
    fn current(&self, slots: &dyn Reservations) -> bool {
        slots.open_peer(self.ticket, self.candidate.peer())
    }
}
pub struct Bindings {
    pub attachment: Attachment,
    pub registration: Box<dyn Registration>,
    pub entropy: Entropy,
    pub work: Box<dyn Work>,
}
pub struct Session {
    request: Request,
    display: Option<Display>,
    // Retain original native capabilities through cancelled startup cleanup.
    _unregistered: Option<Box<dyn Registration>>,
    work: Box<dyn Work>,
    closing: bool,
}
impl Session {
    pub fn candidate(&self) -> &Candidate {
        &self.request.candidate
    }
    pub fn ticket(&self) -> Ticket {
        self.request.ticket
    }
    pub fn closing(&self) -> bool {
        self.closing
    }
    pub fn poll(&mut self, now: Instant) -> Result<()> {
        if !self.work.alive() {
            self.close();
        }
        let Some(display) = &mut self.display else {
            return Ok(());
        };
        let result = display.poll(now);
        if display.state() == State::Retired {
            self.close();
        }
        result
    }
    pub fn state(&self) -> State {
        if self.closing || !self.work.alive() {
            if self.retired() {
                State::Retired
            } else {
                State::Retiring
            }
        } else {
            self.display
                .as_ref()
                .map_or(State::Retiring, Display::state)
        }
    }
    pub fn status(&self) -> Option<TabletSession> {
        (self.state() == State::Active).then(|| TabletSession {
            identity: Identity::UsbPeer(self.request.candidate.peer().clone()),
            instance: self.request.ticket.instance(),
            video_port: self.request.ports.0,
            input_port: self.request.ports.1,
        })
    }
    pub fn close(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        if let Some(display) = &mut self.display {
            display.close();
        }
        self.work.close();
    }
    pub fn retired(&self) -> bool {
        self.closing
            && self
                .display
                .as_ref()
                .is_none_or(|display| display.state() == State::Retired)
            && self.work.retired()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
