//! T917: translate portable capacity decisions into existing ADB retirement.
use super::*;
impl Monitor {
    pub(super) fn preempt_for_usb(&mut self) {
        let DisplayPlan::Replace(ticket) = self.slots.plan() else {
            return;
        };
        let Some(DisplayOwner::Adb(serial)) = self.slots.owner(ticket).cloned() else {
            return;
        };
        if self.current.as_ref() == Some(&serial) {
            self.change_primary(&None);
        } else {
            self.remove_assignment(&serial);
        }
    }
}
#[cfg(test)]
mod tests;
