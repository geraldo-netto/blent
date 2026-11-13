//! T1422: one backend admission retains original constructors and device identity.
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::{
    collections::HashSet,
    io,
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

pub(super) const MAX_BACKENDS: usize = 4096;
pub(super) type OskObservations =
    Arc<Mutex<blent_config::retirement::observations::Observations<2>>>;

static IDENTITIES: LazyLock<Mutex<HashSet<u32>>> = LazyLock::new(Mutex::default);

pub(super) struct Lifetime {
    admission: Option<Admission>,
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    claimed: bool,
    identity: Option<Identity>,
    native: Option<Completion>,
    osk: OskObservations,
    commands: Arc<crate::native_commands::ObservedCommands>,
    payload: blent_config::retirement::observations::Observations<1>,
}
struct Identity(u32);
impl Identity {
    fn claim(instance: u32) -> io::Result<Self> {
        if !IDENTITIES.lock().unwrap().insert(instance) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Input identity still owned",
            ));
        }
        Ok(Self(instance))
    }
}
impl Drop for Identity {
    fn drop(&mut self) {
        IDENTITIES.lock().unwrap().remove(&self.0);
    }
}
impl Lifetime {
    pub(super) fn new() -> io::Result<Arc<Self>> {
        let admission = crate::process_retirement::PROCESS.reserve(pool()?)?;
        Ok(Arc::new(Self {
            admission: Some(admission),
            state: Mutex::new(State::default()),
        }))
    }
    pub(super) fn completion(&self) -> Completion {
        self.admission.as_ref().unwrap().completion()
    }
    pub(super) fn claim(&self, instance: u32) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.claimed {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Input follower already used",
            ));
        }
        state.identity = Some(Identity::claim(instance)?);
        state.claimed = true;
        Ok(())
    }
    pub(super) fn retain(&self, proof: Completion) {
        let mut state = self.state.lock().unwrap();
        assert!(
            state.native.as_ref().is_none_or(Completion::is_complete),
            "original constructor remains owned"
        );
        state.native = Some(proof);
    }
    pub(super) fn observe_osk(&self, proof: Completion) -> io::Result<()> {
        observe_osk(&self.state.lock().unwrap().osk, proof)
    }
    pub(super) fn commands(&self) -> Arc<crate::native_commands::ObservedCommands> {
        self.state.lock().unwrap().commands.clone()
    }
    pub(super) fn admit_payload(&self) -> io::Result<super::payload::Live> {
        let mut state = self.state.lock().unwrap();
        let observation = state.payload.reserve()?;
        let live = super::payload::Live::new(state.commands.clone(), state.osk.clone())?;
        observation.bind(live.completion());
        Ok(live)
    }
}
pub(super) fn observe_osk(observations: &OskObservations, proof: Completion) -> io::Result<()> {
    observations.lock().unwrap().reserve()?.bind(proof);
    Ok(())
}
fn pool() -> io::Result<&'static Pool> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(blent_config::retirement_limits::configured(
            "BLENT_INPUT_LIMIT",
            64,
            MAX_BACKENDS,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))
}
impl Observe for State {
    fn poll_complete(&mut self) -> bool {
        self.commands.complete()
            && self.payload.complete()
            && self.osk.lock().unwrap().complete()
            && self.native.as_ref().is_none_or(Completion::is_complete)
    }
}
impl Drop for Lifetime {
    fn drop(&mut self) {
        let state = std::mem::take(self.state.get_mut().unwrap());
        self.admission.take().unwrap().retain(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1433_osk_observation_exhaustion_rejects_without_forgetting_originals() {
        let originals = Pool::new(3).unwrap();
        let first = originals.reserve().unwrap();
        let second = originals.reserve().unwrap();
        let next = originals.reserve().unwrap();
        let lifetime = Lifetime::new().unwrap();
        lifetime.observe_osk(first.completion()).unwrap();
        lifetime.observe_osk(second.completion()).unwrap();
        assert!(lifetime.observe_osk(next.completion()).is_err());
        assert!(!lifetime
            .state
            .lock()
            .unwrap()
            .osk
            .lock()
            .unwrap()
            .complete());
        drop(first);
        lifetime.observe_osk(next.completion()).unwrap();
        drop((second, next));
        assert!(lifetime
            .state
            .lock()
            .unwrap()
            .osk
            .lock()
            .unwrap()
            .complete());
        let proof = lifetime.completion();
        drop(lifetime);
        while !proof.is_complete() {
            std::thread::yield_now();
        }
    }
}
