//! Portable coordination of selected connections, capacity and joined owners.
//! Native composition retains each admission while startup is in flight. An
//! error may release it only after proving no native work or exposure remains.
use crate::{
    connection_discovery::{Candidate, Discovery},
    display_slots::{Owner, Plan, Slots, Ticket},
    registered_session::{Bindings, Request, Session},
};
use anyhow::{ensure, Result};
use blent_config::tablets::TabletSession;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub enum Decision {
    None,
    Wait,
    Replace(Ticket),
    Start(Admission),
}
pub struct Admission {
    candidate: Candidate,
    ticket: Ticket,
    request: Request,
}
impl Admission {
    pub fn instance(&self) -> u32 {
        self.ticket.instance()
    }
}
struct Starting {
    candidate: Candidate,
    ticket: Ticket,
}
pub struct Connections {
    source: Discovery,
    ports: Vec<(u16, u16)>,
    retry: Duration,
    candidates: Vec<Candidate>,
    starting: BTreeMap<u32, Starting>,
    sessions: Vec<Session>,
    retries: BTreeMap<String, Option<Instant>>,
    closed: bool,
}
impl Connections {
    pub fn new(source: Discovery, ports: Vec<(u16, u16)>, retry: Duration) -> Result<Self> {
        ensure!(
            (1..=4).contains(&ports.len()),
            "invalid connection capacity"
        );
        ensure!(
            !retry.is_zero() && retry <= Duration::from_secs(60),
            "invalid connection retry"
        );
        let mut unique = std::collections::BTreeSet::new();
        for &(video, control) in &ports {
            ensure!(
                video != 0 && control != 0 && unique.insert(video) && unique.insert(control),
                "invalid connection ports"
            );
        }
        Ok(Self {
            source,
            ports,
            retry,
            candidates: vec![],
            starting: BTreeMap::new(),
            sessions: vec![],
            retries: BTreeMap::new(),
            closed: false,
        })
    }
    pub fn poll(&mut self, slots: &mut Slots, now: Instant) -> Result<()> {
        let discovery = self.source.poll(now);
        self.candidates = discovery.as_ref().map_or_else(|_| vec![], Clone::clone);
        self.invalidate_startup(slots);
        let mut error = discovery.err();
        for session in &mut self.sessions {
            if let Err(failure) = observe(session, &self.source, self.closed, slots, now) {
                error.get_or_insert(failure);
            }
        }
        self.reap(slots, now);
        self.retries.retain(|peer, _| {
            self.candidates
                .iter()
                .any(|candidate| candidate.peer().as_str() == peer)
        });
        error.map_or(Ok(()), Err)
    }
    fn invalidate_startup(&self, slots: &mut Slots) {
        for start in self.starting.values() {
            if self.closed || !self.source.current(&start.candidate) {
                let _ = slots.close(start.ticket);
            }
        }
    }
    fn reap(&mut self, slots: &mut Slots, now: Instant) {
        let mut index = 0;
        while index < self.sessions.len() {
            if !self.sessions[index].retired() {
                index += 1;
                continue;
            }
            let session = self.sessions.swap_remove(index);
            let ticket = session.ticket();
            self.retries.insert(
                session.candidate().peer().as_str().to_owned(),
                now.checked_add(self.retry),
            );
            // Drop pinned listeners before capacity can be reused.
            drop(session);
            let _ = slots.retired(ticket);
        }
    }
    pub fn plan(&mut self, slots: &mut Slots, now: Instant) -> Result<Decision> {
        if self.closed {
            slots.select(None);
            return Ok(Decision::None);
        }
        let candidate = self
            .candidates
            .iter()
            .find(|candidate| self.eligible(slots, candidate, now))
            .cloned();
        let Some(candidate) = candidate else {
            slots.select(None);
            return Ok(Decision::None);
        };
        slots.select(Some(candidate.peer().clone()));
        match slots.plan() {
            Plan::Ready(instance) => self
                .reserve(slots, candidate, instance)
                .map(Decision::Start),
            Plan::Replace(ticket) => Ok(Decision::Replace(ticket)),
            _ => Ok(Decision::Wait),
        }
    }
    fn eligible(&self, slots: &Slots, candidate: &Candidate, now: Instant) -> bool {
        slots
            .ticket(&Owner::Peer(candidate.peer().clone()))
            .is_none()
            && self
                .retries
                .get(candidate.peer().as_str())
                .is_none_or(|deadline| deadline.is_some_and(|deadline| now >= deadline))
            && self.source.current(candidate)
    }
    fn reserve(
        &mut self,
        slots: &mut Slots,
        candidate: Candidate,
        instance: u32,
    ) -> Result<Admission> {
        let ports = *self
            .ports
            .get(instance as usize)
            .ok_or_else(|| anyhow::anyhow!("connection capacity mismatch"))?;
        let ticket = slots.reserve(Owner::Peer(candidate.peer().clone()), instance)?;
        let request = Request::new(slots, candidate.clone(), ticket, ports, self.retry)
            .expect("validated reserved connection");
        self.starting.insert(
            instance,
            Starting {
                candidate: candidate.clone(),
                ticket,
            },
        );
        Ok(Admission {
            candidate,
            ticket,
            request,
        })
    }
    /// Infallible ownership transfer, including cancelled or obsolete startup.
    pub fn bind(&mut self, slots: &mut Slots, admission: Admission, bindings: Bindings) {
        self.finish_start(&admission);
        let mut session = admission.request.bind(slots, bindings);
        if self.closed || !self.source.current(&admission.candidate) {
            session.close();
        }
        if session.closing() {
            let _ = slots.close(session.ticket());
        }
        self.sessions.push(session);
    }
    /// Native adapter guarantees failure before exposure and complete cleanup.
    pub fn failed(&mut self, slots: &mut Slots, admission: Admission, now: Instant) {
        self.finish_start(&admission);
        self.retries.insert(
            admission.candidate.peer().as_str().to_owned(),
            now.checked_add(self.retry),
        );
        let _ = slots.close(admission.ticket);
        let _ = slots.retired(admission.ticket);
    }
    fn finish_start(&mut self, admission: &Admission) {
        if self
            .starting
            .get(&admission.instance())
            .is_some_and(|start| start.ticket == admission.ticket)
        {
            self.starting.remove(&admission.instance());
        }
    }
    pub fn status(&self) -> Vec<TabletSession> {
        self.sessions.iter().filter_map(Session::status).collect()
    }
    pub fn close(&mut self, slots: &mut Slots) {
        self.closed = true;
        slots.select(None);
        self.source.close();
        self.invalidate_startup(slots);
        for session in &mut self.sessions {
            session.close();
            let _ = slots.close(session.ticket());
        }
    }
    pub fn retired(&self) -> bool {
        self.closed && self.source.retired() && self.starting.is_empty() && self.sessions.is_empty()
    }
}
fn observe(
    session: &mut Session,
    source: &Discovery,
    closed: bool,
    slots: &mut Slots,
    now: Instant,
) -> Result<()> {
    if closed
        || slots.closing(session.ticket()) != Some(false)
        || !source.current(session.candidate())
    {
        session.close();
    }
    let result = session.poll(now);
    if session.closing() {
        let _ = slots.close(session.ticket());
    }
    result
}
#[cfg(test)]
mod tests;
