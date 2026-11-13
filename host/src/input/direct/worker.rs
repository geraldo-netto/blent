//! One thread owns all native devices. Queue admission never executes native work.
use super::*;

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
            let _ = state.retire();
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
    let _ = state.retire(); // T1403 separately retains failed native release.
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
    let result = execute(state, current, message.command, &message.authority);
    if result.is_err() {
        let _ = state.retire();
    }
    publish(gate, &message.authority, &state.status);
    if let Some(reply) = message.reply {
        if reply.send(result).is_err() {
            message.authority.cancel();
        }
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
