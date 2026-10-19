//! Bounded original source obligations; gates are sponsored only by test callers.
use super::*;
use blent_config::{host_activity::*, retirement::Completion};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
};

#[derive(Clone, Copy)]
pub(in crate::auxiliary_source) enum Mode {
    Ready,
    Failed,
    Panicked,
    Late,
}
pub(in crate::auxiliary_source) struct Original {
    pub epoch: u64,
    pub entered: AtomicBool,
    pub stopped: AtomicBool,
    pub dropped: AtomicBool,
    pub completion: Mutex<Option<Completion>>,
    events: Mutex<VecDeque<HostState>>,
}
impl Original {
    pub fn event(&self, power: Power) {
        let mut events = self.events.lock().unwrap();
        assert!(events.len() < 4);
        events.push_back(HostState {
            power: Some(power),
            ..Default::default()
        });
    }
}
struct State {
    originals: Mutex<Vec<Arc<Original>>>,
    gates: Mutex<VecDeque<mpsc::Receiver<()>>>,
}
pub(in crate::auxiliary_source) struct Factory {
    state: Arc<State>,
    mode: Mode,
}
impl Factory {
    pub fn new(mode: Mode) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(State {
                originals: Mutex::new(Vec::new()),
                gates: Mutex::new(VecDeque::new()),
            }),
            mode,
        })
    }
    pub fn count(&self) -> usize {
        self.state.originals.lock().unwrap().len()
    }
    pub fn original(&self, index: usize) -> Arc<Original> {
        self.state.originals.lock().unwrap()[index].clone()
    }
    pub fn hold(&self) -> mpsc::Sender<()> {
        let (release, gate) = mpsc::channel();
        let mut gates = self.state.gates.lock().unwrap();
        assert!(gates.len() < 4);
        gates.push_back(gate);
        release
    }
}
impl SourceFactory for Factory {
    fn create(&self, settings: SourceSettings, lifetime: SourceLifetime) -> SourceCreation {
        let state = self.state.clone();
        let mode = self.mode;
        Box::pin(async move {
            settings.validate()?;
            match mode {
                Mode::Ready => ready(state, lifetime),
                Mode::Failed => anyhow::bail!("T1081 native source unavailable"),
                Mode::Panicked => panic!("T1081 native creator panic"),
                Mode::Late => late(state, lifetime).await,
            }
        })
    }
}
fn original(state: &State) -> Arc<Original> {
    static EPOCH: AtomicU64 = AtomicU64::new(0);
    let epoch = EPOCH
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .unwrap()
        + 1;
    let original = Arc::new(Original {
        epoch,
        entered: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
        dropped: AtomicBool::new(false),
        completion: Mutex::new(None),
        events: Mutex::new(VecDeque::new()),
    });
    original.event(Power::Running);
    let mut all = state.originals.lock().unwrap();
    assert!(all.len() < 8);
    all.push(original.clone());
    original
}
fn held(
    original: Arc<Original>,
    gate: mpsc::Receiver<()>,
    lifetime: &SourceLifetime,
) -> Result<Task> {
    let observed = original.clone();
    let mut task = Task::spawn_blocking(move || {
        observed.entered.store(true, Ordering::Release);
        let _ = gate.recv();
        Ok(())
    })?;
    let proof = task.completion();
    lifetime.retain(proof.clone());
    task.keep_alive(lifetime.clone());
    *original.completion.lock().unwrap() = Some(proof);
    Ok(task)
}
fn ready(state: Arc<State>, lifetime: SourceLifetime) -> Result<StartedSource> {
    let original = original(&state);
    let gate = state.gates.lock().unwrap().pop_front();
    let task = gate
        .map(|gate| held(original.clone(), gate, &lifetime))
        .transpose()?;
    Ok(StartedSource {
        epoch: original.epoch,
        source: Box::new(Native {
            original,
            revision: 0,
            _task: task,
        }),
    })
}
async fn late(state: Arc<State>, lifetime: SourceLifetime) -> Result<StartedSource> {
    let original = original(&state);
    let gate = state
        .gates
        .lock()
        .unwrap()
        .pop_front()
        .context("T1081 late creation gate")?;
    let (reply, response) = oneshot::channel();
    let observed = original.clone();
    let certificate = original.clone();
    let mut task = Task::spawn_blocking(move || {
        observed.entered.store(true, Ordering::Release);
        let _ = gate.recv();
        let source = StartedSource {
            epoch: original.epoch,
            source: Box::new(Native {
                original,
                revision: 0,
                _task: None,
            }),
        };
        let _ = reply.send(source);
        Ok(())
    })?;
    let proof = task.completion();
    lifetime.retain(proof.clone());
    task.keep_alive(lifetime);
    *certificate.completion.lock().unwrap() = Some(proof);
    task.await??;
    response
        .await
        .context("T1081 original late result withdrawn")
}
// Native callbacks are confined to one original source. Returning true here is
// deliberately only logical completion; SourceLifetime retains real held tasks.
struct Native {
    original: Arc<Original>,
    revision: u64,
    _task: Option<Task>,
}
impl Source for Native {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            power: true,
            ..Default::default()
        }
    }
    fn poll(&mut self) -> Result<Option<Event>> {
        let Some(state) = self.original.events.lock().unwrap().pop_front() else {
            return Ok(None);
        };
        self.revision = self
            .revision
            .checked_add(1)
            .context("T1081 revisions exhausted")?;
        Ok(Some(Event {
            epoch: self.original.epoch,
            revision: self.revision,
            state,
        }))
    }
    fn stop(&mut self) {
        self.original.stopped.store(true, Ordering::Release);
    }
    fn retired(&self) -> bool {
        self.original.stopped.load(Ordering::Acquire)
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.stop();
        self.original.dropped.store(true, Ordering::Release);
    }
}
