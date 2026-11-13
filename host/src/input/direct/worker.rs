//! One thread owns all native devices. Queue admission never executes native work.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// A thread exit alone does not prove its fallible native obligations settled.
pub(super) struct Retiring {
    thread: blent_config::retirement_work::Thread,
    settled: Arc<AtomicBool>,
}
impl Retiring {
    pub(super) fn new(handle: thread::JoinHandle<()>, settled: Arc<AtomicBool>) -> Self {
        Self {
            thread: blent_config::retirement_work::Thread::new(handle),
            settled,
        }
    }
}
impl blent_config::retirement::Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        self.thread.poll_complete() && self.settled.load(Ordering::Acquire)
    }
}

pub(super) fn serve<E: Environment>(
    mut state: state::State<E>,
    receiver: mpsc::Receiver<Request>,
    gate: Arc<Mutex<Gate>>,
) {
    let mut current: Option<Arc<Authority>> = None;
    loop {
        if current
            .as_ref()
            .is_some_and(|authority| authority.cancelled())
        {
            settle(&mut state, &gate);
            current = None;
        }
        if gate.lock().unwrap().closed {
            break;
        }
        match receiver.recv() {
            Ok(Request::Command(message)) => apply(&mut state, &mut current, message, &gate),
            Ok(Request::Wake) => {}
            Err(_) => break,
        }
    }
    settle(&mut state, &gate);
}

/// Retry on the creating thread, retaining State and its bounded admission.
/// Liveness requires eventual adapter acceptance; elapsed time cannot settle debt.
fn settle<E: Environment>(state: &mut state::State<E>, gate: &Mutex<Gate>) {
    let mut reported = false;
    gate.lock().unwrap().cleanup_pending = state.session.is_some();
    while let Err(error) = protected(|| state.retire()) {
        if !reported {
            tracing::warn!("Direct input cleanup retained until native release: {error}");
            reported = true;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
    gate.lock().unwrap().cleanup_pending = false;
}

/// Keep State outside unwinding of fallible adapter commands/retirement.
fn protected(operation: impl FnOnce() -> Result<()>) -> Result<()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
        .unwrap_or_else(|_| Err(anyhow::anyhow!("Native input operation panicked")))
}
fn apply<E: Environment>(
    state: &mut state::State<E>,
    current: &mut Option<Arc<Authority>>,
    message: Message,
    gate: &Mutex<Gate>,
) {
    if message.authority.cancelled() {
        return;
    }
    let result = protected(|| execute(state, current, message.command, &message.authority));
    let failed = result.is_err();
    if result.is_err() {
        state.status.negotiated = false;
        message.authority.cancel();
    }
    publish(gate, &message.authority, &state.status);
    if let Some(reply) = message.reply {
        if reply.send(result).is_err() {
            message.authority.cancel();
        }
    }
    if failed {
        settle(state, gate);
        *current = None;
    }
}
fn execute<E: Environment>(
    state: &mut state::State<E>,
    current: &mut Option<Arc<Authority>>,
    command: Command,
    authority: &Arc<Authority>,
) -> Result<()> {
    if current
        .as_ref()
        .is_none_or(|old| !Arc::ptr_eq(old, authority))
    {
        state.retire()?;
        *current = Some(authority.clone());
    }
    ensure!(!authority.cancelled(), "Input command revoked");
    state.command(command)
}
fn publish(gate: &Mutex<Gate>, authority: &Arc<Authority>, status: &Status) {
    let mut gate = gate.lock().unwrap();
    if Arc::ptr_eq(&gate.current, authority) && !authority.cancelled() {
        gate.status = status.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blent_config::retirement::Observe;

    #[test]
    fn t1403_thread_exit_without_native_settlement_cannot_release_admission() {
        let settled = Arc::new(AtomicBool::new(false));
        let mut retiring = Retiring::new(thread::spawn(|| ()), settled.clone());
        let start = std::time::Instant::now();
        while !retiring.thread.poll_complete() {
            assert!(start.elapsed() < std::time::Duration::from_secs(3));
            thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(
            !retiring.poll_complete(),
            "T1403 thread exit substituted for cleanup acceptance"
        );
        settled.store(true, Ordering::Release);
        assert!(retiring.poll_complete());
    }
}
