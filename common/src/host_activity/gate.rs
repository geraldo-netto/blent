use super::*;
use crate::retirement::Completion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub lifetime: u64,
    pub request: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Idle,
    Start { lifetime: u64 },
    Keep(Ticket),
    Stop(Ticket),
    Waiting(Ticket),
}
struct Owned {
    ticket: Ticket,
    completion: Completion,
    closing: bool,
}
impl Owned {
    fn plan(&mut self, wanted: bool, selected: Option<u64>, authority: &dyn Authority) -> Plan {
        if self.closing {
            return Plan::Waiting(self.ticket);
        }
        if !wanted
            || selected != Some(self.ticket.lifetime)
            || !authority.current(self.ticket.lifetime)
        {
            self.closing = true;
            return Plan::Stop(self.ticket);
        }
        Plan::Keep(self.ticket)
    }
}
/// Reserve actual startup/native ownership before exposing a factory. Logical
/// Stop remains irreversible through rapid resume; no timer can free ownership.
/// Instantiate separately for display, camera, microphone and speaker resources.
#[derive(Default)]
pub struct Gate {
    pub(super) next: u64,
    owner: Option<Owned>,
}
impl Gate {
    fn completed(&mut self) {
        if self
            .owner
            .as_ref()
            .is_some_and(|owner| owner.completion.is_complete())
        {
            self.owner = None;
        }
    }
    pub fn plan(
        &mut self,
        wanted: bool,
        selected: Option<u64>,
        authority: &dyn Authority,
    ) -> Result<Plan> {
        ensure!(selected != Some(0), "invalid original resource lifetime");
        self.completed();
        if let Some(owner) = &mut self.owner {
            return Ok(owner.plan(wanted, selected, authority));
        }
        if let Some(lifetime) = selected.filter(|lifetime| wanted && authority.current(*lifetime)) {
            ensure!(
                self.next < u64::MAX,
                "activity resource request counters exhausted"
            );
            return Ok(Plan::Start { lifetime });
        }
        Ok(Plan::Idle)
    }
    /// Completion belongs to an independently retained startup/native owner,
    /// not a visibility flag. Backend rechecks plan/authority before acquisition.
    pub fn reserve(
        &mut self,
        wanted: bool,
        lifetime: u64,
        completion: Completion,
        authority: &dyn Authority,
    ) -> Result<Ticket> {
        ensure!(
            self.plan(wanted, Some(lifetime), authority)? == Plan::Start { lifetime },
            "original activity resource cannot start"
        );
        ensure!(
            !completion.is_complete(),
            "activity startup ownership already retired"
        );
        self.next += 1; // plan checked exhaustion without wrapping.
        let ticket = Ticket {
            lifetime,
            request: self.next,
        };
        self.owner = Some(Owned {
            ticket,
            completion,
            closing: false,
        });
        Ok(ticket)
    }
}
