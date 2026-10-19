//! T1109: suppress unchanged pictures between bounded recovery refreshes.
//! The adapter supplies current original-source observations, never native fences.
use super::{Phase as CadencePhase, Sample};

pub const REFRESH_MS: u32 = 1000;
const WINDOW: u8 = 16;

#[derive(Clone, Copy, Debug, Default)]
pub struct Conditions {
    pub original_live: bool,
    pub viewers_present: bool,
    pub liveness_ready: bool,
}
impl Conditions {
    fn permits(self) -> bool {
        self.original_live && self.viewers_present && self.liveness_ready
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Baseline,
    SparseTrial,
    UnchangedTrial,
    Active,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    First,
    Damage,
    Cursor,
    Refresh,
    Unchanged,
}

/// Owns one original session's bounded qualification state. Construct only for
/// an explicitly selected suppression mode. Dropping this value grants no native
/// retirement; adapters retain their own source, request and original completion.
pub struct Policy {
    cadence: super::Policy,
    unchanged_samples: u8,
    last_tick_us: Option<u64>,
}
impl Policy {
    pub fn new(qualified_recipe: bool, named_decoder: bool) -> Self {
        Self {
            cadence: super::Policy::new(qualified_recipe && named_decoder),
            unchanged_samples: 0,
            last_tick_us: None,
        }
    }

    pub fn phase(&self) -> Phase {
        match self.cadence.phase {
            CadencePhase::Baseline => Phase::Baseline,
            CadencePhase::Trial => Phase::SparseTrial,
            CadencePhase::Active if self.unchanged_samples == WINDOW => Phase::Active,
            CadencePhase::Active => Phase::UnchangedTrial,
            CadencePhase::Rejected => Phase::Rejected,
        }
    }

    pub fn interval_ms(&self) -> u32 {
        if self.cadence.phase == CadencePhase::Active {
            REFRESH_MS
        } else {
            self.cadence.interval_ms()
        }
    }

    /// The caller must already hold a current frame. First publication, damage,
    /// cursor changes and explicit refresh requests bypass unchanged-frame delay.
    pub fn emit(&self, change: Change, elapsed_ms: u64) -> bool {
        change != Change::Unchanged || elapsed_ms >= u64::from(self.interval_ms())
    }

    pub fn reject(&mut self) {
        self.cadence.reject();
        self.unchanged_samples = 0;
    }

    pub fn observe(&mut self, conditions: Conditions, frame: Sample) {
        if !conditions.permits() {
            self.reject();
            return;
        }
        // Reuse the single authoritative ACK/cost/keyframe/sequence history.
        let previous = self.cadence.previous;
        self.cadence.observe(frame);
        self.qualify_refresh(previous, frame);
    }

    fn qualify_refresh(&mut self, previous: Option<Sample>, frame: Sample) {
        if self.cadence.phase != CadencePhase::Active {
            self.unchanged_samples = 0;
            return;
        }
        if self.unchanged_samples == WINDOW {
            return; // Fresh motion does not undo already matched qualification.
        }
        let gap = previous.and_then(|old| frame.pts_us.checked_sub(old.pts_us));
        if gap.is_some_and(|gap| (850_000..=1_250_000).contains(&gap)) {
            self.unchanged_samples += 1;
        } else {
            self.unchanged_samples = 0;
        }
    }

    /// One monotonic clock; delayed ACK processing may precede this tick, but
    /// an ACK from the future or a backward tick cannot renew a native request.
    pub fn tick(&mut self, conditions: Conditions, now_us: u64) {
        if !conditions.permits() || !self.clock_valid(now_us) {
            self.reject();
            return;
        }
        self.last_tick_us = Some(now_us);
        self.cadence.tick(now_us);
        if self.cadence.phase == CadencePhase::Rejected {
            self.unchanged_samples = 0;
        }
    }

    fn clock_valid(&self, now_us: u64) -> bool {
        self.last_tick_us.is_none_or(|last| now_us >= last)
            && self
                .cadence
                .previous
                .is_none_or(|frame| now_us >= frame.ack_us)
    }
}

#[cfg(test)]
mod tests;
