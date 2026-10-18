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
mod activity;
use activity::Activity;

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
        let mut display = current.then(|| {
            Display::new(
                self.candidate.peer().clone(),
                bindings.attachment,
                registration.take().expect("unexposed registration binding"),
                bindings.entropy,
                self.retry,
            )
            .expect("validated USB display request")
        });
        let activity = display.as_mut().and_then(|display| {
            bindings
                .activity
                .map(|binding| Activity::new(binding, display))
        });
        let mut session = Session {
            activity,
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
    pub activity: Option<ActivityBinding>,
    pub attachment: Attachment,
    pub registration: Box<dyn Registration>,
    pub entropy: Entropy,
    pub work: Box<dyn Work>,
}
/// Original display runtime capabilities and shared machine observation. Bind
/// the controller with its own actual whole-runtime completion before exposure.
/// This contains no native backend representation or auxiliary media authority.
#[derive(Clone)]
pub struct ActivityBinding {
    pub control: crate::display_quiescence::Control,
    pub retirement: blent_config::retirement::Completion,
    pub source: Option<tokio::sync::watch::Receiver<blent_config::host_activity::Publication>>,
}
pub struct Session {
    activity: Option<Activity>,
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
        self.reconcile_activity()?;
        let result = self
            .display
            .as_mut()
            .map_or(Ok(()), |display| display.poll(now));
        self.reconcile_activity()?;
        if self
            .display
            .as_ref()
            .is_some_and(|display| display.state() == State::Retired)
        {
            self.close();
        }
        result
    }
    fn reconcile_activity(&mut self) -> Result<()> {
        if self.closing {
            return Ok(());
        }
        let (Some(activity), Some(display)) = (&mut self.activity, &mut self.display) else {
            return Ok(());
        };
        if let Err(error) = activity.reconcile(display) {
            self.close();
            return Err(error);
        }
        Ok(())
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
            if let Some(activity) = &mut self.activity {
                activity.close(display);
            }
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
            && self.activity.as_ref().is_none_or(Activity::retired)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
