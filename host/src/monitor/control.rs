//! The monitor owns admitted intent after caller loss. IPC jobs own bytes only;
//! exact display tickets remain with the existing native retirement workflows.
use super::*;
use blent_config::{
    display_control::{Command, Phase, Reply, Snapshot},
    linux::display_control::{Call, Server},
};

pub(super) struct Control {
    owner: String,
    server: Server,
}
impl Control {
    pub(super) fn bind() -> Result<Self> {
        let configured = std::env::var_os("BLENT_DISPLAY_CONTROL_CLIENTS");
        let capacity = match configured {
            Some(value) => parse_capacity(
                value
                    .to_str()
                    .context("display control capacity must be UTF-8")?,
            )?,
            None => 8,
        };
        Ok(Self {
            owner: blent_config::credentials::random_token()?,
            server: Server::bind(capacity)?,
        })
    }
    pub(super) async fn close(self) {
        self.server.close().await;
    }
}

fn parse_capacity(value: &str) -> Result<usize> {
    let capacity = value.parse()?;
    anyhow::ensure!(
        (1..=64).contains(&capacity),
        "display control capacity must be 1..64"
    );
    Ok(capacity)
}

pub(super) async fn event(control: &mut Option<Control>) -> Result<Call> {
    let Some(control) = control else {
        return std::future::pending().await;
    };
    throttle(control.server.next().await).await
}

async fn throttle(result: Result<Call>) -> Result<Call> {
    if result.is_err() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    result
}

fn snapshot(state: &Monitor, owner: &str) -> Snapshot {
    let mut assignments = state.slots.assignments();
    let sessions = state.sessions();
    for assignment in &mut assignments {
        if assignment.phase == Phase::Starting
            && sessions.iter().any(|session| {
                session.instance == assignment.lease.slot
                    && blent_config::display_control::Target::from_identity(&session.identity)
                        == assignment.target
            })
        {
            assignment.phase = Phase::Assigned;
        }
    }
    Snapshot {
        owner: owner.to_owned(),
        assignments,
    }
}

fn apply(state: &mut Monitor, owner: &str, command: &Command) -> Result<()> {
    command.validate()?;
    let Command::Set {
        owner: expected_owner,
        target,
        enabled,
        expected,
    } = command
    else {
        return Ok(());
    };
    anyhow::ensure!(
        expected_owner == owner,
        "display service changed; refresh tablet state"
    );
    let target = DisplayOwner::from_target(target)?;
    if *enabled {
        anyhow::ensure!(
            state.slots.ticket(&target).is_none(),
            "original display lease still exists"
        );
        return state.slots.set_enabled(target, true);
    }
    let ticket = state
        .slots
        .ticket(&target)
        .context("display lease retired; refresh tablet state")?;
    anyhow::ensure!(
        Some(ticket.lease()) == *expected,
        "display lease changed; refresh tablet state"
    );
    state.slots.set_enabled(target.clone(), false)?;
    state.slots.close(ticket)?;
    if let DisplayOwner::Adb(serial) = target {
        stop_adb(state, &serial);
    }
    Ok(())
}

fn stop_adb(state: &mut Monitor, serial: &str) {
    if state.current.as_deref() == Some(serial) {
        state.change_primary(&None);
    } else {
        state.remove_assignment(serial);
    }
}

pub(super) fn handle(state: &mut Monitor, call: Result<Call>) {
    let call = match call {
        Ok(call) => call,
        Err(error) => {
            warn!("Display control transport: {error:#}");
            return;
        }
    };
    let Some(owner) = state.control.as_ref().map(|control| control.owner.clone()) else {
        return;
    };
    let error = apply(state, &owner, &call.command)
        .err()
        .map(|error| format!("{error:#}"));
    let reply = Reply {
        snapshot: snapshot(state, &owner),
        error,
    };
    state.control.as_mut().unwrap().server.respond(call, reply);
}

#[cfg(test)]
mod tests;
