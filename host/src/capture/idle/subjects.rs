//! Borrowed observations of one continuous viewer group; no native authority.
use super::{worker, EncoderEvidence};
use blent::capture_contract::{Signal, ViewerActivity, ViewerPresence};
use blent_config::idle::{unchanged::Conditions, Mode};
use std::sync::Arc;

pub(super) struct Subjects {
    viewers: Arc<dyn ViewerActivity>,
    original: Option<ViewerPresence>,
    epoch: u64,
}
impl Subjects {
    pub(super) fn new(mode: Mode, viewers: Arc<dyn ViewerActivity>) -> Self {
        let epoch = viewers.epoch();
        let original = if mode == Mode::Unchanged {
            viewers.presence()
        } else {
            None
        };
        Self {
            viewers,
            original,
            epoch,
        }
    }
    pub(super) fn same_epoch(&self) -> bool {
        self.viewers.epoch() == self.epoch
    }
    fn continuous(&self) -> bool {
        self.original.as_ref().is_some_and(|original| {
            self.viewers
                .presence()
                .is_some_and(|current| current.continues(original))
        })
    }
    pub(super) fn ready(&self) -> bool {
        self.same_epoch() && self.continuous() && self.viewers.liveness_ready()
    }
    pub(super) fn current(
        &self,
        mode: Mode,
        evidence: &EncoderEvidence,
        stop: &worker::Stop,
    ) -> Conditions {
        if mode != Mode::Unchanged {
            return Conditions::default(); // Legacy cadence has no liveness dependency.
        }
        Conditions {
            original_live: evidence.active() && stop.active(),
            viewers_present: self.same_epoch() && self.continuous(),
            liveness_ready: self.viewers.liveness_ready(),
        }
    }
    pub(super) fn permits(
        &self,
        mode: Mode,
        evidence: &EncoderEvidence,
        stop: &worker::Stop,
    ) -> bool {
        match mode {
            Mode::Compatible => false,
            Mode::Adaptive => evidence.active() && stop.active() && self.same_epoch(),
            Mode::Unchanged => self.current(mode, evidence, stop).permits(),
        }
    }
    /// Arm before inspecting current observations. Dropping this future retains
    /// no membership or native grant; the worker owns this read-only source.
    pub(super) fn changed(&self, mode: Mode) -> Signal<'static> {
        if mode != Mode::Unchanged {
            return Box::pin(std::future::pending());
        }
        self.viewers.changed()
    }
}
