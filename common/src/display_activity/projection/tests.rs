use super::*;
use crate::host_activity::*;

fn capabilities() -> Capabilities {
    Capabilities {
        power: true,
        lock: true,
        display: true,
    }
}
fn observe(feed: &mut Feed, revision: u64, locked: bool) {
    feed.observe(Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(locked),
            display_on: Some(true),
        },
    })
    .unwrap();
}
fn publication(feed: &Feed) -> Option<Publication> {
    Some(Publication::Observed {
        capabilities: capabilities(),
        snapshot: feed.snapshot(true, false),
    })
}
#[test]
fn t1055_coalesced_stop_publishes_new_fence_without_replaying_old_work() {
    let mut owner = Projection::default();
    assert_eq!(
        owner
            .observe(Some(Publication::Pending), true)
            .unwrap()
            .unwrap(),
        Signal::new(1, 0, false).unwrap()
    );
    let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
    observe(&mut feed, 1, false);
    assert_eq!(
        owner.observe(publication(&feed), true).unwrap().unwrap(),
        Signal::new(2, 0, true).unwrap()
    );
    observe(&mut feed, 2, true);
    observe(&mut feed, 3, false);
    assert_eq!(
        owner.observe(publication(&feed), true).unwrap().unwrap(),
        Signal::new(3, 1, true).unwrap()
    );
    assert!(owner.observe(publication(&feed), true).unwrap().is_none());
    observe(&mut feed, 4, false);
    assert!(owner.observe(publication(&feed), true).unwrap().is_none());
    assert_eq!(
        owner.observe(publication(&feed), false).unwrap().unwrap(),
        Signal::new(4, 2, false).unwrap()
    );
    assert!(owner.observe(publication(&feed), false).unwrap().is_none());
}
#[test]
fn t1055_absent_source_and_unavailable_publications_have_explicit_policy() {
    let mut owner = Projection::default();
    assert!(owner.observe(None, false).unwrap().unwrap().active());
    for source in [
        Publication::Pending,
        Publication::Failed,
        Publication::Stopped,
    ] {
        assert!(!owner.observe(Some(source), true).unwrap().unwrap().active());
        assert!(owner.observe(None, false).unwrap().unwrap().active());
    }
}
#[test]
fn t1055_exhaustion_is_terminal_and_never_wraps_or_reinitializes_original_owner() {
    let mut owner = Projection {
        last: Some(Signal::new(MAX_REVISION - 1, 0, false).unwrap()),
        ..Default::default()
    };
    assert_eq!(
        owner.observe(None, false).unwrap().unwrap().revision(),
        MAX_REVISION
    );
    assert!(owner.observe(None, false).unwrap().is_none());
    assert!(owner.observe(Some(Publication::Failed), true).is_err());
    assert!(owner.failed);
    assert!(owner.original.is_none());
    assert!(owner.observe(None, false).is_err());
    assert_eq!(owner.last.unwrap().revision(), MAX_REVISION);
}

struct Model {
    active: bool,
    emitted: bool,
    revision: u64,
    stopped: u64,
}
impl Model {
    fn step(&mut self, next: bool, interrupted: bool) -> Option<Signal> {
        let stop = self.active && interrupted;
        if self.emitted && self.active == next && !stop {
            return None;
        }
        self.emitted = true;
        self.revision += 1;
        self.stopped += u64::from(stop);
        self.active = next;
        Some(Signal::new(self.revision, self.stopped, self.active).unwrap())
    }
}
fn input(feed: &mut Feed, command: u64, revision: u64) -> (Option<Publication>, bool, bool) {
    match command {
        0 => {
            observe(feed, revision, false);
            (publication(feed), true, false)
        }
        1 => {
            observe(feed, revision, true);
            (publication(feed), false, true)
        }
        2 => {
            observe(feed, revision, true);
            observe(feed, revision + 1, false);
            (publication(feed), true, true)
        }
        _ => (Some(Publication::Failed), false, true),
    }
}
#[test]
fn t1055_exhaustive_five_step_projection_traces_match_independent_stop_model() {
    for trace in 0..1024u64 {
        let mut commands = trace;
        let mut owner = Projection::default();
        let mut feed = Feed::new(1, capabilities(), Settings::default()).unwrap();
        let mut model = Model {
            active: false,
            emitted: false,
            revision: 0,
            stopped: 0,
        };
        for step in 0..5 {
            let (publication, active, interrupted) = input(&mut feed, commands % 4, step * 2 + 1);
            commands /= 4;
            assert_eq!(
                owner.observe(publication, true).unwrap(),
                model.step(active, interrupted),
                "T1055 trace {trace} step {step}"
            );
        }
    }
}
