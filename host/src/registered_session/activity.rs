//! Join current demand with original broker/native ownership; never free by time.
use super::{ActivityBinding, Display, State};
use crate::{activity_view::View, display_quiescence::Fence};
use anyhow::{ensure, Result};
use blent_config::host_activity::{ActivityTarget, WorkObservation};

pub(super) struct Activity {
    binding: ActivityBinding,
    source: View,
    original: Option<WorkObservation>,
    fence: Option<Fence>,
}
impl Activity {
    pub(super) fn new(binding: ActivityBinding, display: &mut Display) -> Self {
        let source = View::new(ActivityTarget::Display, binding.source.clone());
        let mut activity = Self {
            binding,
            source,
            original: None,
            fence: None,
        };
        // Even a first USB handoff must await original primary/extra capture.
        activity.pause(display);
        activity
    }
    pub(super) fn reconcile(&mut self, display: &mut Display) -> Result<()> {
        self.invalidate(display);
        self.prepare(display);
        self.release(display)
    }
    fn invalidate(&mut self, display: &mut Display) {
        let continuous = self
            .original
            .is_some_and(|original| self.source.permits(original));
        let registered = self.fence.is_some() || display.state() == State::Active;
        if !continuous || !registered {
            self.pause(display);
            self.original = None;
        }
    }
    fn prepare(&mut self, display: &mut Display) {
        if self.original.is_some() || display.state() != State::Paused {
            return;
        }
        let Some(original) = self.source.begin() else {
            return;
        };
        if !self.fence.as_ref().is_some_and(Fence::complete) {
            return;
        }
        self.original = Some(original);
        display.resume();
    }
    fn release(&mut self, display: &Display) -> Result<()> {
        if self.original.is_none() || display.state() != State::Active {
            return Ok(());
        }
        let Some(fence) = &self.fence else {
            return Ok(());
        };
        let mut resumed = false;
        display.apply(|| resumed = self.binding.control.resume(fence));
        ensure!(resumed, "original display native quiescence unavailable");
        self.fence = None;
        Ok(())
    }
    fn pause(&mut self, display: &mut Display) {
        if self.fence.is_none() {
            display.apply(|| self.fence = Some(self.binding.control.pause()));
        }
        display.pause();
    }
    pub(super) fn close(&mut self, display: &mut Display) {
        self.pause(display);
        self.original = None;
    }
    pub(super) fn retired(&self) -> bool {
        self.binding.retirement.is_complete() || self.fence.as_ref().is_some_and(Fence::complete)
    }
}
