//! Per-resource local I/O ownership. No local endpoint is opened before admission.
use super::{
    pump::Pump,
    session::{Session, State},
};
use crate::accessory::Stream;
use anyhow::{ensure, Result};
use blent_config::connection::{
    management::{Credential, Profile, Reason, Resource},
    resources::Lease,
    wire::Channel,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite};

/// Adapter connects only descriptor-validated local endpoints. Cancellation must
/// synchronously drop ALL native I/O owned by this future; no detached workers.
/// Returned streams obey the same rule. Process-backed adapters need a separate
/// retirement contract and cannot use this synchronous socket adapter boundary.
pub type Connect = Pin<Box<dyn Future<Output = Result<Vec<Box<dyn Stream>>>> + Send>>;
struct Entry {
    lease: Lease,
    connect: Option<Connect>,
    admitted: bool,
    closing: bool,
    pumps: Vec<(Channel, Pump<Box<dyn Stream>>)>,
}
pub struct Offer {
    pub resource: Resource,
    pub owner: u64,
    pub credential: Credential,
    pub profile: Profile,
    pub connect: Connect,
}
pub struct Bridges {
    entries: [Option<Entry>; 4],
    block: usize,
    timeout: Duration,
}
impl Bridges {
    pub fn new(block: usize, timeout: Duration) -> Result<Self> {
        ensure!(
            (1..=65_536).contains(&block),
            "invalid local bridge capacity"
        );
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(60),
            "invalid local bridge timeout"
        );
        Ok(Self {
            entries: std::array::from_fn(|_| None),
            block,
            timeout,
        })
    }
    pub fn register<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        session: &mut Session<S>,
        offer: Offer,
        now: i64,
        timeout: i64,
    ) -> Result<Lease> {
        let index = offer.resource as usize - 1;
        ensure!(self.entries[index].is_none(), "resource still owned");
        let lease = session.register(
            offer.resource,
            offer.owner,
            offer.credential,
            offer.profile,
            now,
            timeout,
        )?;
        self.entries[index] = Some(Entry {
            lease: lease.clone(),
            connect: Some(offer.connect),
            admitted: false,
            closing: false,
            pumps: Vec::new(),
        });
        Ok(lease)
    }
    /// Apply every Session state notice before exposing it to local clients.
    pub fn state<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        session: &mut Session<S>,
        lease: &Lease,
        state: State,
    ) -> Result<()> {
        let slot = &mut self.entries[lease.key.resource as usize - 1];
        let entry = slot
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("unknown local resource"))?;
        ensure!(entry.lease == *lease, "stale local resource notice");
        match state {
            State::PermissionRequired => (),
            State::Admitted => {
                ensure!(session.current(lease), "local resource not admitted");
                ensure!(
                    !entry.closing && !entry.admitted,
                    "duplicate or closed admission"
                );
                let connect = entry
                    .connect
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("missing local connector"))?;
                let timeout = self.timeout;
                entry.connect = Some(Box::pin(async move {
                    tokio::time::timeout(timeout, connect).await?
                }));
                entry.admitted = true;
            }
            State::Closing => retire(session, entry)?,
            State::Retired => {
                ensure!(entry.closing, "premature resource retirement");
                *slot = None;
            }
        }
        Ok(())
    }
    /// Poll after local readiness AND every wire progress event. Each invocation
    /// gives all four resources a bounded turn, even when a sibling is pending.
    pub fn poll<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        session: &mut Session<S>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<()>> {
        let mut progressed = false;
        for entry in self.entries.iter_mut().flatten() {
            if !entry.admitted || entry.closing {
                continue;
            }
            match entry.poll(session, cx, self.block, self.timeout) {
                Poll::Pending => (),
                Poll::Ready(Ok(())) => progressed = true,
                Poll::Ready(Err(_)) => {
                    session.cancel(&entry.lease, Reason::WorkerFailed)?;
                    retire(session, entry)?;
                    progressed = true;
                }
            }
        }
        if progressed {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    pub fn disconnect<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        session: &mut Session<S>,
    ) -> Result<()> {
        session.disconnect()?;
        for entry in self.entries.iter_mut().flatten() {
            retire(session, entry)?;
        }
        Ok(())
    }
}
impl Entry {
    fn poll<S: AsyncRead + AsyncWrite + Unpin>(
        &mut self,
        session: &mut Session<S>,
        cx: &mut Context<'_>,
        block: usize,
        timeout: Duration,
    ) -> Poll<Result<()>> {
        if let Some(connect) = self.connect.as_mut() {
            let streams = std::task::ready!(connect.as_mut().poll(cx))?;
            self.connect.take();
            self.install(streams, block, timeout)?;
        }
        let mut progressed = false;
        for (channel, pump) in &mut self.pumps {
            let mut endpoint = session.endpoint(&self.lease, *channel);
            progressed |= pump.poll(cx, &mut endpoint)?.is_ready();
        }
        if progressed {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn install(
        &mut self,
        streams: Vec<Box<dyn Stream>>,
        block: usize,
        timeout: Duration,
    ) -> Result<()> {
        let channels = members(self.lease.key.resource);
        ensure!(streams.len() == channels.len(), "wrong local stream count");
        self.pumps = streams
            .into_iter()
            .zip(channels)
            .map(|(stream, channel)| {
                Ok((
                    *channel,
                    Pump::new(stream, self.lease.clone(), *channel, block, timeout)?,
                ))
            })
            .collect::<Result<_>>()?;
        Ok(())
    }
}
fn retire<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut Session<S>,
    entry: &mut Entry,
) -> Result<()> {
    if entry.closing {
        return Ok(());
    }
    entry.connect.take();
    entry.pumps.clear();
    session.native_retired(&entry.lease)?;
    entry.closing = true;
    Ok(())
}
fn members(resource: Resource) -> &'static [Channel] {
    match resource {
        Resource::DisplaySession => &[Channel::Display, Channel::Control],
        Resource::Camera => &[Channel::Camera],
        Resource::Microphone => &[Channel::Microphone],
        Resource::Speakers => &[Channel::Speaker],
    }
}

#[cfg(test)]
mod tests;
