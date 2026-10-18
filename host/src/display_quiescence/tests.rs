use super::*;

#[test]
fn t1050_all_original_participants_and_exact_request_are_required() {
    let (control, participants) = channel(2).unwrap();
    let source = participants[0].source();
    let original = source.begin().unwrap();
    let fence = control.pause();
    assert!(!source.permits(&original));
    assert!(source.begin().is_none());
    assert!(!control.resume(&fence));
    assert!(participants[0].acknowledge(&participants[0].pending().unwrap()));
    assert!(!fence.complete());
    assert!(!control.resume(&fence));
    assert!(participants[1].acknowledge(&participants[1].pending().unwrap()));
    assert!(fence.complete());
    assert!(control.resume(&fence));
    assert!(!fence.complete());
    assert!(!source.permits(&original));
    assert!(source.begin().is_some());
}

#[test]
fn t1050_foreign_stale_and_closed_owners_cannot_complete_or_resume() {
    let (control, participants) = channel(1).unwrap();
    let (other, other_participants) = channel(1).unwrap();
    let fence = control.pause();
    let old = participants[0].pending().unwrap();
    let foreign = other.pause();
    assert!(!participants[0].acknowledge(&other_participants[0].pending().unwrap()));
    assert!(!control.resume(&foreign));
    participants[0].acknowledge(&old);
    assert!(control.resume(&fence));
    assert!(!participants[0].acknowledge(&old));
    let next = control.pause();
    assert!(!participants[0].acknowledge(&old));
    assert!(!control.resume(&fence));
    assert!(!next.complete());
    participants[0].acknowledge(&participants[0].pending().unwrap());
    assert!(next.complete());
    drop(participants);
    assert!(!next.complete());
    assert!(!control.resume(&next));
}

#[tokio::test]
async fn t1050_original_notices_survive_coalescing_and_source_loss() {
    let (control, participants) = channel(1).unwrap();
    let source = participants[0].source();
    let original = source.begin().unwrap();
    let notice = source.changed();
    let fence = control.pause();
    let repeated = control.pause();
    participants[0].acknowledge(&participants[0].pending().unwrap());
    assert!(repeated.complete());
    assert!(control.resume(&fence));
    notice.await;
    assert!(!source.permits(&original));
    let notice = source.changed();
    drop(control);
    notice.await;
    assert!(source.begin().is_none());
    assert!(participants[0].pending().is_none());
    assert!(!fence.complete());
    let mut notice = source.changed();
    assert!(futures_util::poll!(&mut notice).is_pending());
}

#[test]
fn t1050_capacity_bounds_and_repeated_generations_keep_receipts_scoped() {
    for count in [0, 3, usize::MAX] {
        assert!(channel(count).is_err());
    }
    for count in 1..=2 {
        let (control, participants) = channel(count).unwrap();
        for _ in 0..128 {
            let fence = control.pause();
            for participant in &participants {
                participant.acknowledge(&participant.pending().unwrap());
            }
            assert!(control.resume(&fence));
            assert!(!control.resume(&fence));
        }
    }
}

struct Model {
    open: bool,
    acknowledged: [bool; 2],
    original_revoked: bool,
}
struct Trace {
    control: Control,
    participants: Vec<Participant>,
    fence: Option<Fence>,
    source: Source,
    original: Permit,
    model: Model,
}
impl Trace {
    fn new() -> Self {
        let (control, participants) = channel(2).unwrap();
        let source = participants[0].source();
        let original = source.begin().unwrap();
        Self {
            control,
            participants,
            fence: None,
            source,
            original,
            model: Model {
                open: true,
                acknowledged: [false; 2],
                original_revoked: false,
            },
        }
    }
    fn pause(&mut self) {
        self.fence = Some(self.control.pause());
        if self.model.open {
            self.model.open = false;
            self.model.acknowledged = [false; 2];
        }
        self.model.original_revoked = true;
    }
    fn acknowledge(&mut self, index: usize) {
        if let Some(pause) = self.participants[index].pending() {
            assert!(self.participants[index].acknowledge(&pause));
        }
        if !self.model.open {
            self.model.acknowledged[index] = true;
        }
    }
    fn resume(&mut self) {
        let Some(fence) = &self.fence else {
            return;
        };
        let allowed = !self.model.open && self.model.acknowledged.iter().all(|ready| *ready);
        assert_eq!(self.control.resume(fence), allowed);
        if allowed {
            self.model.open = true;
        }
    }
    fn step(&mut self, action: u32) {
        match action {
            0 => self.pause(),
            1 => self.acknowledge(0),
            2 => self.acknowledge(1),
            _ => self.resume(),
        }
    }
    fn verify(&self, pattern: u32, step: u32) {
        assert_eq!(
            self.source.begin().is_some(),
            self.model.open,
            "T1050 trace={pattern} step={step}"
        );
        assert_eq!(
            self.source.permits(&self.original),
            !self.model.original_revoked
        );
        if let Some(fence) = &self.fence {
            assert_eq!(
                fence.complete(),
                !self.model.open && self.model.acknowledged.iter().all(|ready| *ready)
            );
        }
    }
}
#[test]
fn t1050_exhaustive_bounded_command_traces_match_independent_admission_model() {
    for pattern in 0..1024u32 {
        let mut trace = Trace::new();
        for step in 0..5 {
            trace.step((pattern >> (2 * step)) & 3);
            trace.verify(pattern, step);
        }
    }
}
