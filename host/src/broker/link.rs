//! One independently cancellable physical link; bounded control and notice lanes.
use super::{
    bridges::{Bridges, Offer},
    connection::Connection,
    session::{Event as MediaEvent, Limits, State},
    window,
};
use crate::{
    accessory::{
        setup::{self, Verified},
        Resource,
    },
    owned_task::Task,
    secure_connection::Identity,
};
use anyhow::{ensure, Context, Result};
use blent_config::{
    connection::{
        pairing::{PeerId, Ticket},
        resources::{AdmissionTimeout, Lease},
    },
    retirement::Completion,
};
use std::{
    sync::Arc,
    task::{Context as PollContext, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
    time::Instant,
};
mod start;

pub enum Command {
    Approve(Ticket),
    Admit(Connection),
    Register(Offer),
    Cancel(Lease),
}
pub enum Event {
    Identified(PeerId),
    Verified(Verified, Limits),
    Registered { owner: u64, result: Result<Lease> },
    State(Lease, State),
}
pub struct Handle {
    commands: mpsc::Sender<Command>,
    events: mpsc::Receiver<Event>,
    task: Task,
    native: Completion,
    finished: bool,
}
pub struct Settings {
    pub identity: Arc<Identity>,
    pub limits: Limits,
    pub block: usize,
    pub setup_timeout: Duration,
    pub approval_timeout: Duration,
    pub resource_timeout: AdmissionTimeout,
    pub origin: Instant,
}
impl Handle {
    /// Admission spans blocking process creation, queued startup results and
    /// eventual native helper exit. Cancelling the async wait cannot free it.
    pub fn start_factory(
        pool: &blent_config::retirement::Pool,
        factory: impl FnOnce() -> Result<Resource> + Send + 'static,
        settings: Settings,
    ) -> Result<Self> {
        let admission = pool.reserve()?;
        let native = admission.completion();
        let (commands, input) = mpsc::channel(8);
        let (output, events) = mpsc::channel(32);
        let task = Task::spawn(async move {
            let resource =
                tokio::task::spawn_blocking(move || start::open(admission, factory)).await??;
            run(resource, settings, input, output).await
        })?;
        Ok(Self {
            commands,
            events,
            task,
            native,
            finished: false,
        })
    }
    /// Resource is already admitted; caller attaches native Completion to its
    /// controller before allowing discovery callbacks to change pairing state.
    pub fn start(resource: Resource, settings: Settings) -> Result<Self> {
        let native = resource.owner.completion();
        let (commands, input) = mpsc::channel(8);
        let (output, events) = mpsc::channel(32);
        let task = Task::spawn(run(resource, settings, input, output))?;
        Ok(Self {
            commands,
            events,
            task,
            native,
            finished: false,
        })
    }
    pub fn submit(&self, command: Command) -> std::result::Result<(), Command> {
        self.commands
            .try_send(command)
            .map_err(|error| error.into_inner())
    }
    pub fn stop(&self) {
        self.task.abort();
    }
    pub fn native_completion(&self) -> Completion {
        self.native.clone()
    }
    pub fn task_completion(&self) -> Completion {
        self.task.completion()
    }
    /// Terminal task error means all owned local streams were dropped. Native
    /// helper completion remains separate and must still gate replacement.
    pub async fn next(&mut self) -> Result<Event> {
        ensure!(!self.finished, "link already retired");
        tokio::select! {
            event = self.events.recv() => match event {
                Some(event) => Ok(event),
                None => {
                    let result = (&mut self.task).await;
                    self.finished = true;
                    result??;
                    anyhow::bail!("link notice lane closed")
                }
            },
            result = &mut self.task => { self.finished = true; result??; anyhow::bail!("link exited") }
        }
    }
}
fn emit(output: &mpsc::Sender<Event>, event: Event) -> Result<()> {
    output
        .try_send(event)
        .map_err(|_| anyhow::anyhow!("link notice capacity exhausted"))
}
async fn run(
    resource: Resource,
    settings: Settings,
    mut input: mpsc::Receiver<Command>,
    output: mpsc::Sender<Event>,
) -> Result<()> {
    let candidate = setup::discover(resource, &settings.identity, settings.setup_timeout).await?;
    emit(&output, Event::Identified(candidate.peer().clone()))?;
    let Some(Command::Approve(ticket)) = input.recv().await else {
        anyhow::bail!("expected exact pairing approval");
    };
    let mut verified = candidate
        .authenticate_with_timeout(&settings.identity, ticket, settings.approval_timeout)
        .await?;
    let limits = window::negotiate(&mut verified.resource.stream, settings.limits).await?;
    emit(&output, Event::Verified(verified, limits))?;
    let Some(Command::Admit(mut connection)) = input.recv().await else {
        anyhow::bail!("expected authenticated lease admission");
    };
    let mut bridges = Bridges::new(settings.block, limits.io_timeout)?;
    let result = media(
        &mut connection,
        &mut bridges,
        &mut input,
        &output,
        settings.origin,
        settings.resource_timeout,
    )
    .await;
    let _ = bridges.disconnect(connection.session());
    let _ = connection.retire().await;
    result
}
async fn media(
    connection: &mut Connection,
    bridges: &mut Bridges,
    input: &mut mpsc::Receiver<Command>,
    output: &mpsc::Sender<Event>,
    origin: Instant,
    resource_timeout: AdmissionTimeout,
) -> Result<()> {
    let mut clock = tokio::time::interval(Duration::from_millis(20));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            command = input.recv() => dispatch(connection.session(), bridges, command.context("local link owner gone")?, output, now(origin)?, resource_timeout)?,
            _ = clock.tick() => connection.session().tick(now(origin)?)?,
            event = std::future::poll_fn(|cx| poll_media(connection, bridges, cx, origin)) => {
                if let MediaEvent::State(lease, state) = event? { emit(output, Event::State(lease, state))?; }
            }
        }
    }
}
fn dispatch<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut super::session::Session<S>,
    bridges: &mut Bridges,
    command: Command,
    output: &mpsc::Sender<Event>,
    now: i64,
    resource_timeout: AdmissionTimeout,
) -> Result<()> {
    match command {
        Command::Register(offer) => {
            let owner = offer.owner;
            let result = bridges.register(session, offer, now, resource_timeout.milliseconds());
            emit(output, Event::Registered { owner, result })
        }
        Command::Cancel(lease) => cancel_original(session, &lease),
        _ => anyhow::bail!("unexpected setup command during media"),
    }
}
pub(crate) use super::session::cancel_original;

fn poll_media(
    connection: &mut Connection,
    bridges: &mut Bridges,
    cx: &mut PollContext<'_>,
    origin: Instant,
) -> Poll<Result<MediaEvent>> {
    let session = connection.session();
    let progressed = bridges.poll(session, cx)?.is_ready();
    match session.poll_event(cx, now(origin)?) {
        Poll::Ready(Ok(MediaEvent::State(lease, state))) => {
            bridges.state(session, &lease, state)?;
            Poll::Ready(Ok(MediaEvent::State(lease, state)))
        }
        Poll::Pending if progressed => Poll::Ready(Ok(MediaEvent::Progress)),
        result => result,
    }
}
pub fn now(origin: Instant) -> Result<i64> {
    let elapsed = origin.elapsed().as_millis();
    ensure!(
        elapsed <= i64::MAX as u128,
        "broker monotonic time exhausted"
    );
    Ok(elapsed as i64)
}

#[cfg(test)]
mod tests;
