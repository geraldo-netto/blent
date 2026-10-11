//! T872 host resource orchestration over an already authenticated transport.
//! The owner serializes commands and checks the current pairing before creation.
//! No native endpoint, UI or persistence operation runs inside media dispatch.
use super::transport::{Event as FrameEvent, Transport};
use anyhow::{ensure, Result};
use blent_config::connection::{
    management::{Credential, Decoder, Key, Message, Outcome, Profile, Reason, Resource},
    mux::Multiplexer,
    pairing::Ticket,
    resources::{Install, Lease, Resources, Role},
    wire::{Channel, Header, Kind, AUTHENTICATED_SESSION, MAX_COUNTER, MAX_CREDIT, MAX_PAYLOAD},
};
use std::{
    collections::VecDeque,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite};
mod cancellation;
pub(crate) use cancellation::cancel_original;
mod channels;
mod endpoint;
mod lifecycle;
mod management;
mod readiness;
pub use blent_config::connection::endpoint::Read;
use channels::Channels;
pub use endpoint::Endpoint;
use management::{Effect, Publication};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    PermissionRequired,
    Admitted,
    Closing,
    Retired,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Progress,
    State(Lease, State),
    Readable(Key, Channel),
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub channel_bytes: u32,
    pub queued_messages: usize,
    pub quantum: u32,
    pub io_timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            channel_bytes: 262_144,
            queued_messages: 64,
            quantum: MAX_PAYLOAD,
            io_timeout: Duration::from_secs(5),
        }
    }
}
struct Binding {
    lease: Lease,
    installed: bool,
    closing: bool,
    native_retired: bool,
    channels_retired: bool,
    acknowledged: bool,
    deferred: Option<Message>,
}
pub struct Session<S> {
    transport: Option<Transport<S>>,
    resources: Resources,
    slots: [Option<Binding>; 4],
    channels: Channels,
    decoder: Decoder,
    publications: VecDeque<Publication>,
    active: Option<Publication>,
    events: VecDeque<Event>,
    capabilities: Option<u8>,
    readable: [Option<Key>; 5],
    readable_cursor: usize,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    /// `ticket` must come from the current stock-TLS authenticated Pairing lease.
    /// No wire value, registration or saved approval alone satisfies this boundary.
    /// `limits.channel_bytes` must be agreed with the peer over TLS before either
    /// side sends CREDIT; v1 uses a symmetric window. T853/T854 own setup policy.
    pub fn new(stream: S, ticket: Ticket, limits: Limits) -> Result<Self> {
        ensure!(
            (4096..=MAX_CREDIT).contains(&limits.channel_bytes),
            "invalid runtime window"
        );
        let mux = Multiplexer::new(
            AUTHENTICATED_SESSION,
            1,
            limits.channel_bytes,
            limits.queued_messages,
            limits.quantum,
        )?;
        let transport = Transport::new(stream, mux, limits.io_timeout)?;
        Ok(Self {
            transport: Some(transport),
            resources: Resources::new(Role::Host, ticket)?,
            slots: [None, None, None, None],
            channels: Channels::new(limits.channel_bytes),
            decoder: Decoder::new(),
            publications: VecDeque::new(),
            active: None,
            events: VecDeque::new(),
            capabilities: None,
            readable: [None; 5],
            readable_cursor: 0,
        })
    }
    fn transport(&mut self) -> Result<&mut Transport<S>> {
        self.transport
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("link disconnected"))
    }
    fn binding(&mut self, lease: &Lease) -> Result<&mut Binding> {
        let binding = self.slots[lease.key.resource as usize - 1]
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("resource not registered"))?;
        ensure!(binding.lease == *lease, "stale resource callback");
        Ok(binding)
    }
    pub fn current(&self, lease: &Lease) -> bool {
        self.resources.current(lease)
    }
    pub fn capabilities(&self) -> Option<u8> {
        self.capabilities
    }
    pub fn register(
        &mut self,
        resource: Resource,
        owner: u64,
        credential: Credential,
        profile: Profile,
        now: i64,
        timeout: i64,
    ) -> Result<Lease> {
        self.transport()?;
        self.room()?;
        // Validate before reserving Resources or mutating local ownership.
        let mut profile_bytes = [0; 256];
        profile.encode_for(resource, &mut profile_bytes)?;
        let lease = self.resources.begin(resource, owner, now, timeout)?;
        self.slots[resource as usize - 1] = Some(Binding {
            lease: lease.clone(),
            installed: false,
            closing: false,
            native_retired: false,
            channels_retired: false,
            acknowledged: false,
            deferred: None,
        });
        self.queue(Message::Offer(lease.key, credential, profile), Effect::None)?;
        Ok(lease)
    }
    pub fn offer(
        &mut self,
        lease: &Lease,
        channel: Channel,
        bytes: Vec<u8>,
    ) -> std::result::Result<(), Vec<u8>> {
        if !self.current(lease) || !channels::belongs(lease.key.resource, channel) {
            return Err(bytes);
        }
        match self.transport.as_mut() {
            Some(transport) => transport.multiplexer().offer(channel, bytes),
            None => Err(bytes),
        }
    }
    /// One bounded consumer borrow per channel. Credit returns only on consumed,
    /// after actual native output completion, not after copying into its buffer.
    pub fn read(
        &mut self,
        lease: &Lease,
        channel: Channel,
        out: &mut [u8],
    ) -> Result<Option<Read>> {
        ensure!(
            self.current(lease) && channels::belongs(lease.key.resource, channel),
            "inactive consumer"
        );
        self.channels.read(channel, out)
    }
    pub fn consumed(&mut self, lease: &Lease, channel: Channel, read: Read) -> Result<()> {
        self.binding(lease)?;
        ensure!(
            channels::belongs(lease.key.resource, channel),
            "wrong resource channel"
        );
        self.channels.consumed(channel, read)?;
        if self.current(lease) {
            self.transport()?
                .multiplexer()
                .release(channel, read.count as u32)?;
            self.mark_readable(lease.key, channel);
        }
        Ok(())
    }
    pub async fn next(&mut self, now: i64) -> Result<Event> {
        let start = tokio::time::Instant::now();
        std::future::poll_fn(|cx| {
            let elapsed = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
            self.poll_event(cx, now.checked_add(elapsed).unwrap_or(-1))
        })
        .await
    }
    /// `now` is the owner's monotonic clock in milliseconds. Call again with a
    /// fresh value after each event; tick drives idle offer deadlines separately.
    pub fn poll_event(&mut self, cx: &mut Context<'_>, now: i64) -> Poll<Result<Event>> {
        let result = self.poll_step(cx, now);
        if matches!(result, Poll::Ready(Err(_))) {
            if let Err(error) = self.disconnect() {
                return Poll::Ready(Err(error));
            }
        }
        result
    }
    fn poll_step(&mut self, cx: &mut Context<'_>, now: i64) -> Poll<Result<Event>> {
        ensure_poll_time(now)?;
        if self.transport.is_none() {
            return Poll::Ready(
                self.next_notice()
                    .ok_or_else(|| anyhow::anyhow!("link disconnected")),
            );
        }
        self.publish()?;
        let event = match self.transport()?.poll_event(cx) {
            Poll::Pending => {
                return self
                    .next_notice()
                    .map_or(Poll::Pending, |event| Poll::Ready(Ok(event)))
            }
            Poll::Ready(result) => result?,
        };
        match event {
            FrameEvent::Written(header) => self.written(header, now)?,
            FrameEvent::Received { header, deliver } => self.received(header, deliver, now)?,
        }
        self.settle()?;
        Poll::Ready(Ok(self.next_notice().unwrap_or(Event::Progress)))
    }
    fn received(&mut self, header: Header, deliver: bool, now: i64) -> Result<()> {
        if header.channel == Channel::Management {
            self.management_frame(header, deliver, now)?;
        } else if matches!(header.kind, Kind::Close | Kind::Reset) {
            self.peer_terminal(header.channel)?;
        } else if header.kind == Kind::Data && deliver {
            self.media_frame(header)?;
        }
        self.transport()?.consume()
    }
    fn peer_terminal(&mut self, channel: Channel) -> Result<()> {
        let resource = channels::resource(channel)?;
        let binding = self.slots[resource as usize - 1]
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("terminal without resource"))?;
        ensure!(binding.installed, "terminal before resource Commit");
        let lease = binding.lease.clone();
        self.cancel(&lease, Reason::WorkerFailed)
    }
    fn media_frame(&mut self, header: Header) -> Result<()> {
        let resource = channels::resource(header.channel)?;
        let binding = self.slots[resource as usize - 1]
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("media without registration"))?;
        ensure!(
            binding.installed && !binding.closing,
            "media resource inactive"
        );
        self.channels.push(
            header.channel,
            self.transport.as_ref().expect("online").payload(),
        )?;
        if self.current(&binding.lease) {
            let key = binding.lease.key;
            self.mark_readable(key, header.channel);
        }
        Ok(())
    }
}
fn ensure_poll_time(now: i64) -> Result<()> {
    ensure!(now >= 0, "invalid monotonic time");
    Ok(())
}
#[cfg(test)]
pub(super) mod tests;
