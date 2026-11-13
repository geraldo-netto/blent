//! T1422: one backend admission retains original constructors and device identity.
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::{
    collections::HashSet,
    io,
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

static IDENTITIES: LazyLock<Mutex<HashSet<u32>>> = LazyLock::new(Mutex::default);

pub(super) struct Lifetime {
    admission: Option<Admission>,
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    claimed: bool,
    identity: Option<Identity>,
    native: Vec<Completion>,
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
        state.native.retain(|proof| !proof.is_complete());
        // One sequential constructor and one final OSK restoration are funded.
        assert!(state.native.len() < 2, "Input native observation capacity");
        state.native.push(proof);
    }
    pub(super) fn restore_touch(&self) {
        match crate::owned_task::Task::spawn(async {
            crate::osk::sync_touch_state(&super::TOUCH_DEVICES).await;
            Ok(())
        }) {
            Ok(task) => {
                self.retain(task.completion());
                tokio::spawn(async move {
                    let _ = task.await;
                });
            }
            Err(error) => {
                tracing::warn!("OSK restoration admission failed: {error}; disk recovery retained")
            }
        }
    }
}
fn pool() -> io::Result<&'static Pool> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(blent_config::retirement_limits::configured(
            "BLENT_INPUT_LIMIT",
            64,
            4096,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))
}
impl Observe for State {
    fn poll_complete(&mut self) -> bool {
        self.native.iter().all(Completion::is_complete)
    }
}
impl Drop for Lifetime {
    fn drop(&mut self) {
        let state = std::mem::take(self.state.get_mut().unwrap());
        self.admission.take().unwrap().retain(state);
    }
}
