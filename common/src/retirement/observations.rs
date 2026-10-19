//! Bounded observation, separate from native ownership and cancellation.
//! Reserve before creation; bind its original completion before yielding or
//! returning the resource. Failed creation drops an unbound reservation. Bound
//! proofs remain owned by this table until actual completion permits slot reuse.
use super::Completion;
use std::{
    io,
    sync::{Arc, Mutex, MutexGuard},
};

enum State {
    Reserved,
    Bound(Completion),
    Empty,
}
impl State {
    fn complete(&self) -> bool {
        match self {
            Self::Reserved => false,
            Self::Bound(proof) => proof.is_complete(),
            Self::Empty => true,
        }
    }
}

/// Owns at most N independent original proof identities. Zero capacity rejects
/// every creation. No background worker, device or native owner is created here.
pub struct Observations<const N: usize> {
    entries: [Option<Arc<Mutex<State>>>; N],
}
impl<const N: usize> Default for Observations<N> {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
        }
    }
}
impl<const N: usize> Observations<N> {
    pub fn reserve(&mut self) -> io::Result<Reservation> {
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.as_ref().is_none_or(|entry| state(entry).complete()));
        let slot = slot.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "Original native observations retained",
            )
        })?;
        // A fresh identity prevents a retained old ticket from addressing a
        // reused slot. Allocation happens before the caller's native effects.
        let original = Arc::new(Mutex::new(State::Reserved));
        *slot = Some(original.clone());
        Ok(Reservation(original))
    }
    pub fn complete(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.as_ref().is_none_or(|entry| state(entry).complete()))
    }
}

/// Linear creator handoff. No cloning, index-based rebinding or fallible bind.
/// The caller must bind the resource's exact original proof synchronously after
/// creation. Dropping before binding means no resource was successfully created.
pub struct Reservation(Arc<Mutex<State>>);
impl Reservation {
    pub fn bind(self, completion: Completion) {
        *state(&self.0) = State::Bound(completion);
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut original = state(&self.0);
        if matches!(*original, State::Reserved) {
            *original = State::Empty;
        }
    }
}
fn state(value: &Mutex<State>) -> MutexGuard<'_, State> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
