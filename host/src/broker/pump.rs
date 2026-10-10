//! T878 bounded local byte pump, independent of sockets and Session internals.
//! The owning reactor polls this after both local I/O readiness and channel
//! progress. Pump owns its stream handle; the native adapter separately retains
//! retirement evidence before releasing the resource lease. Dropping buffers
//! does not prove a separately owned worker has retired.
use anyhow::{ensure, Result};
use blent_config::connection::{
    endpoint::Endpoint,
    resources::Lease,
    wire::{Channel, MAX_CREDIT, MAX_PAYLOAD},
};
use std::{
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite};
mod receive;
mod send;
use receive::Receive;
use send::Send;

pub struct Pump<S> {
    stream: S,
    lease: Lease,
    channel: Channel,
    receive: Receive,
    send: Send,
    failed: bool,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Pump<S> {
    /// 64 KiB is a useful default; capacity is configurable without changing
    /// wire framing. One pending block per direction, no per-chunk async tasks.
    pub fn new(
        stream: S,
        lease: Lease,
        channel: Channel,
        block: usize,
        timeout: Duration,
    ) -> Result<Self> {
        ensure!(
            (1..=MAX_PAYLOAD as usize).contains(&block),
            "invalid bridge block capacity"
        );
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(60),
            "invalid bridge deadline"
        );
        Ok(Self {
            stream,
            lease,
            channel,
            receive: Receive::new(block, timeout),
            send: Send::new(block, timeout),
            failed: false,
        })
    }
    /// Pending state survives cancelled waits. Errors poison the pump: owner
    /// cancels this resource and retires all its local I/O, without dropping a
    /// sibling resource. The same pump must never bind another endpoint/lease.
    pub fn poll(&mut self, cx: &mut Context<'_>, endpoint: &mut dyn Endpoint) -> Poll<Result<()>> {
        let result = self.step(cx, endpoint);
        if matches!(result, Poll::Ready(Err(_))) {
            self.failed = true;
        }
        result
    }
    fn step(&mut self, cx: &mut Context<'_>, endpoint: &mut dyn Endpoint) -> Poll<Result<()>> {
        require(!self.failed && endpoint.active(), "inactive local bridge")?;
        require(
            endpoint.identity() == (&self.lease, self.channel),
            "wrong local bridge lease",
        )?;
        require(
            (1..=MAX_CREDIT as usize).contains(&endpoint.capacity()),
            "invalid endpoint capacity",
        )?;
        let received = self.receive.poll(cx, &mut self.stream, endpoint)?;
        let sent = self.send.poll(cx, &mut self.stream, endpoint)?;
        if received.is_ready() || sent.is_ready() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

fn require(valid: bool, message: &'static str) -> Result<()> {
    ensure!(valid, message);
    Ok(())
}

#[cfg(test)]
mod tests;
