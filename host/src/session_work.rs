//! Runtime-to-lifecycle adapter. Primary capture stays owned by the daemon;
//! extra capture/input runtimes stay owned through actual supervisor retirement.
use crate::{registered_session::Work, session::Runtime, session_state::State};
use tokio::sync::watch;

enum Binding {
    Owned(Runtime),
    Shared(watch::Receiver<State>),
}
pub struct RuntimeWork {
    binding: Binding,
    closed: bool,
}
impl RuntimeWork {
    pub fn owned(runtime: Runtime) -> Self {
        Self {
            binding: Binding::Owned(runtime),
            closed: false,
        }
    }
    pub fn shared(runtime: &Runtime) -> Self {
        Self::shared_status(runtime.status.clone())
    }
    /// Composition can retain an observer without retaining the daemon runtime.
    pub fn shared_status(status: watch::Receiver<State>) -> Self {
        Self {
            binding: Binding::Shared(status),
            closed: false,
        }
    }
    fn running(&self) -> bool {
        match &self.binding {
            Binding::Owned(runtime) => matches!(*runtime.status.borrow(), State::Running),
            Binding::Shared(status) => matches!(*status.borrow(), State::Running),
        }
    }
}
impl Work for RuntimeWork {
    fn alive(&self) -> bool {
        !self.closed && self.running()
    }
    fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        if let Binding::Owned(runtime) = &self.binding {
            runtime.stop_tx.send_replace(true);
        }
    }
    fn retired(&self) -> bool {
        self.closed
            && match &self.binding {
                Binding::Owned(runtime) => runtime.retirement.is_complete() && runtime.finished(),
                Binding::Shared(_) => true,
            }
    }
}
impl Drop for RuntimeWork {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests;
