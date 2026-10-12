//! Bounded portable source publication. Never coalesce away a pause/resume edge.
use super::*;
use std::collections::VecDeque;

pub struct Observations {
    epoch: u64,
    pub(super) revision: u64,
    capabilities: Capabilities,
    capacity: usize,
    state: HostState,
    events: VecDeque<Event>,
    failed: bool,
    stopped: bool,
}
impl Observations {
    pub fn new(epoch: u64, capabilities: Capabilities, capacity: usize) -> Result<Self> {
        ensure!(epoch > 0, "invalid host observation epoch");
        ensure!(
            (1..=256).contains(&capacity),
            "invalid host observation capacity"
        );
        Ok(Self {
            epoch,
            revision: 0,
            capabilities,
            capacity,
            state: HostState::default(),
            events: VecDeque::new(),
            failed: false,
            stopped: false,
        })
    }
    pub fn current(&self) -> HostState {
        self.state
    }
    pub fn publish(&mut self, state: HostState) -> Result<bool> {
        ensure!(!self.stopped && !self.failed, "host observations closed");
        policy::validate(state, self.capabilities)?;
        if self.revision != 0 && self.state == state {
            return Ok(false);
        }
        if self.revision == u64::MAX {
            self.failed = true;
            self.events.clear();
            anyhow::bail!("host observation revision exhausted");
        }
        self.revision += 1;
        if self.events.len() == self.capacity {
            self.failed = true;
            self.events.clear();
            self.append(HostState::default());
            anyhow::bail!("host observation queue exhausted");
        }
        self.append(state);
        Ok(true)
    }
    fn append(&mut self, state: HostState) {
        self.state = state;
        self.events.push_back(Event {
            epoch: self.epoch,
            revision: self.revision,
            state,
        });
    }
    pub fn poll(&mut self) -> Result<Option<Event>> {
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        ensure!(
            !self.failed || self.stopped,
            "host observation source requires replacement"
        );
        Ok(None)
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.events.clear();
    }
}
