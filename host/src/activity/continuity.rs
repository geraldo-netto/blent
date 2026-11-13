//! One factory remembers capability admission, never observations or native owners.
use super::*;

/// Initially absent backends are explicitly unsupported. Once observed, their
/// temporary absence must fail creation so the follower remains conservative and
/// retries only after the rejected candidate's original native lifetime settles.
/// Three booleans live with the factory; clones borrow the same admission history.
#[derive(Clone, Default)]
pub(super) struct Continuity(Arc<Mutex<Capabilities>>);

impl Continuity {
    pub(super) fn accept(&self, current: Capabilities) -> Result<()> {
        let mut known = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let retained = (!known.power || current.power)
            && (!known.lock || current.lock)
            && (!known.display || current.display);
        ensure!(
            retained,
            "previously observed native activity capability is unavailable"
        );
        *known = current;
        Ok(())
    }
}
