//! T873 cancellation-safe framed I/O. Authentication and resource admission are
//! the owner's responsibility; wire IDs never grant authority. Polling native
//! AsyncRead/AsyncWrite retains no borrowed payload after each poll returns.
use anyhow::{ensure, Result};
use blent_config::connection::{
    mux::Multiplexer,
    wire::{Channel, Header, HEADER_BYTES, MAX_PAYLOAD},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite};
pub(super) mod deadline;
mod input;
mod output;
use deadline::Deadline;
use input::Input;
use output::Output;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Complete envelope/body in `payload`. Call consume before reading another.
    Received { header: Header, deliver: bool },
    /// Complete header/body and flush; Multiplexer writer borrow has retired.
    Written(Header),
}

/// One fixed receive staging buffer and the mux's existing outbound ownership.
/// Never spawn per-frame tasks or copy outbound payloads. A frame deadline starts
/// at first received byte or selection for writing, never resets on progress.
/// On any I/O/protocol failure the entire link is unusable; its external owner
/// must disconnect Resources and establish actual native retirement separately.
pub struct Transport<S> {
    stream: S,
    mux: Multiplexer,
    input: Input,
    output: Output,
    timeout: Duration,
    failed: bool,
    ready: bool,
    read_first: bool,
    held: [Option<u64>; 5],
}
impl<S: AsyncRead + AsyncWrite + Unpin> Transport<S> {
    pub fn new(stream: S, mux: Multiplexer, timeout: Duration) -> Result<Self> {
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(60),
            "invalid frame deadline"
        );
        Ok(Self {
            stream,
            mux,
            input: Input::new(),
            output: Output::default(),
            timeout,
            failed: false,
            ready: false,
            read_first: false,
            held: [None; 5],
        })
    }
    /// Stop dispatch and transfer stream ownership for bounded TLS shutdown.
    /// No native borrow survives poll return. Dropping/closing this stream does
    /// not establish external helper/worker retirement; that owner stays separate.
    pub fn into_stream(self) -> S {
        self.stream
    }
    /// Serialize lifecycle/credit/queue transitions between events. The caller
    /// must not call next/finish itself: this driver owns writer reservations.
    pub fn multiplexer(&mut self) -> &mut Multiplexer {
        &mut self.mux
    }
    /// Hold a newly committed media generation until whole management-record
    /// publication completes. Management credit must keep flowing meanwhile.
    pub fn hold(&mut self, channel: Channel, generation: Option<u64>) -> Result<()> {
        ensure!(
            channel != Channel::Management,
            "cannot gate management credit"
        );
        ensure!(
            generation.is_none_or(
                |value| (1..=blent_config::connection::wire::MAX_COUNTER).contains(&value)
            ),
            "invalid held generation"
        );
        self.held[channel as usize - 1] = generation;
        Ok(())
    }
    pub fn payload(&self) -> &[u8] {
        if self.ready && !self.failed {
            self.input.payload()
        } else {
            &[]
        }
    }
    pub fn consume(&mut self) -> Result<()> {
        ensure!(self.ready && !self.failed, "no received frame to consume");
        self.ready = false;
        self.input.clear();
        Ok(())
    }
    /// Cancellation-safe: all partial offsets and absolute deadlines live here.
    pub async fn next(&mut self) -> Result<Event> {
        std::future::poll_fn(|cx| self.poll_event(cx)).await
    }
    pub fn poll_event(&mut self, cx: &mut Context<'_>) -> Poll<Result<Event>> {
        if self.failed {
            return Poll::Ready(Err(anyhow::anyhow!("connection failed")));
        }
        let result = self.poll_fair(cx);
        if matches!(result, Poll::Ready(Err(_))) {
            self.failed = true;
            if self.mux.reserved().is_some() {
                let _ = self.mux.finish(false);
            }
        }
        result
    }
    fn poll_fair(&mut self, cx: &mut Context<'_>) -> Poll<Result<Event>> {
        // Alternate completed events; a stream of CREDIT cannot starve writes.
        if self.read_first {
            if let Poll::Ready(event) = self.poll_read(cx) {
                return Poll::Ready(event);
            }
        }
        if let Poll::Ready(event) = self.poll_write(cx) {
            return Poll::Ready(event);
        }
        self.poll_read(cx)
    }
    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<Event>> {
        if self.ready {
            return Poll::Pending;
        }
        let (header, deliver) = std::task::ready!(self.input.poll(
            cx,
            Pin::new(&mut self.stream),
            &mut self.mux,
            self.timeout,
            &self.held
        ))?;
        self.ready = true;
        self.read_first = false;
        Poll::Ready(Ok(Event::Received { header, deliver }))
    }
    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<Event>> {
        let header = std::task::ready!(self.output.poll(
            cx,
            Pin::new(&mut self.stream),
            &mut self.mux,
            self.timeout
        ))?;
        self.read_first = true;
        Poll::Ready(Ok(Event::Written(header)))
    }
}

#[cfg(test)]
mod tests;
