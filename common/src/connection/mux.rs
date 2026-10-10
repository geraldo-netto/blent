//! Single-owner bounded multiplexer. Authentication and native I/O are adapters.
use super::{
    flow::Flow,
    queue::Scheduler,
    wire::{Channel, Header, Kind, CHANNELS},
};
use anyhow::{ensure, Result};

pub struct Frame<'a> {
    pub header: Header,
    pub bytes: &'a [u8],
}

/// One writer reservation and one fixed metadata slot per channel. A failed
/// committed write poisons the connection; possibly transmitted bytes are never
/// refunded. Receive bodies must be read/drained before accepting another header.
pub struct Multiplexer {
    session: u64,
    generations: [u64; CHANNELS as usize],
    capacity: u32,
    consumer_retired: [bool; CHANNELS as usize],
    enabled: [bool; CHANNELS as usize],
    flows: [Flow; CHANNELS as usize],
    queue: Scheduler,
    credit: [u32; CHANNELS as usize],
    terminal: [Option<Header>; CHANNELS as usize],
    closing: [bool; CHANNELS as usize],
    active: Option<Header>,
    metadata_cursor: usize,
    metadata_first: bool,
    failed: bool,
}
impl Multiplexer {
    pub fn new(
        session: u64,
        generation: u64,
        capacity: u32,
        messages: usize,
        quantum: u32,
    ) -> Result<Self> {
        let flows = [
            Flow::new(session, generation, Channel::Control, capacity)?,
            Flow::new(session, generation, Channel::Display, capacity)?,
            Flow::new(session, generation, Channel::Camera, capacity)?,
            Flow::new(session, generation, Channel::Microphone, capacity)?,
            Flow::new(session, generation, Channel::Speaker, capacity)?,
            Flow::new(session, generation, Channel::Management, capacity)?,
        ];
        Ok(Self {
            session,
            generations: [generation; CHANNELS as usize],
            capacity,
            consumer_retired: [false; CHANNELS as usize],
            flows,
            queue: Scheduler::new(capacity, messages, quantum)?,
            credit: [0, 0, 0, 0, 0, capacity],
            enabled: [false, false, false, false, false, true],
            terminal: [None; CHANNELS as usize],
            closing: [false; CHANNELS as usize],
            active: None,
            metadata_cursor: 0,
            metadata_first: true,
            failed: false,
        })
    }
    /// Caller serializes this with authenticated management commit completion.
    /// On the sender, finish the entire COMMIT record before activating; on the
    /// receiver, install the lease and activate while handling that record,
    /// before receiving the next envelope. Wire generation is not authority.
    pub fn activate(&mut self, channel: Channel, generation: u64) -> Result<()> {
        let index = channel as usize - 1;
        ensure!(
            !self.failed && !self.closing[index] && !self.enabled[index]
                && self.generations[index] == generation,
            "channel unavailable or stale activation"
        );
        self.enabled[index] = true;
        self.credit[index] = self.capacity;
        Ok(())
    }
    /// Recheck after asynchronous body completion; revocation can arrive after
    /// header validation and must never revive delivery from cached permission.
    pub fn accepts_data(&self, channel: Channel, generation: u64) -> bool {
        let index = channel as usize - 1;
        !self.failed && self.enabled[index] && !self.closing[index]
            && self.generations[index] == generation
    }
    pub fn offer(&mut self, channel: Channel, bytes: Vec<u8>) -> std::result::Result<(), Vec<u8>> {
        if self.failed || self.closing[channel as usize - 1] || !self.enabled[channel as usize - 1] {
            return Err(bytes);
        }
        self.queue.offer(channel, bytes)
    }
    /// Returns whether DATA belongs to a live consumer. False DATA must be
    /// drained, never delivered. Closure cannot grant authority to a new owner.
    pub fn receive(&mut self, header: Header) -> Result<bool> {
        ensure!(!self.failed, "connection failed");
        let index = header.channel as usize - 1;
        if self.closing[index] {
            self.flows[index].drain(header)?;
            return Ok(false);
        }
        ensure!(
            self.enabled[index] || matches!(header.kind, Kind::Close | Kind::Reset),
            "channel not activated"
        );
        self.flows[index].receive(header)?;
        if matches!(header.kind, Kind::Close | Kind::Reset) {
            self.close(header.channel, header.kind)?;
        }
        Ok(header.kind == Kind::Data)
    }
    pub fn release(&mut self, channel: Channel, bytes: u32) -> Result<()> {
        let index = channel as usize - 1;
        ensure!(!self.failed && !self.closing[index] && self.enabled[index], "channel unavailable");
        self.flows[index].release(bytes)?;
        self.credit[index] += bytes;
        Ok(())
    }
    /// Revoke now; send one ordered terminal after any already reserved payload.
    /// Its final sequence is safe to assign here: closing forbids later DATA or
    /// CREDIT from this channel, and next() preserves the active writer first.
    pub fn close(&mut self, channel: Channel, kind: Kind) -> Result<()> {
        ensure!(!self.failed, "connection failed");
        let index = channel as usize - 1;
        ensure!(!self.closing[index], "channel already closing");
        let terminal = self.flows[index].terminal(kind)?;
        self.terminal[index] = Some(terminal);
        self.closing[index] = true;
        self.enabled[index] = false;
        self.credit[index] = 0;
        if self.active.is_none_or(|active| active.channel != channel) {
            self.queue.clear(channel)?;
        }
        Ok(())
    }
    fn metadata(&mut self) -> Result<Option<Header>> {
        for distance in 0..CHANNELS as usize {
            let index = (self.metadata_cursor + distance) % CHANNELS as usize;
            let header = if let Some(header) = self.terminal[index].take() {
                header
            } else if self.credit[index] > 0 {
                let header = self.flows[index].send(Kind::Credit, 0, self.credit[index])?;
                self.credit[index] = 0;
                header
            } else {
                continue;
            };
            self.metadata_cursor = (index + 1) % CHANNELS as usize;
            self.metadata_first = false;
            self.active = Some(header);
            return Ok(Some(header));
        }
        Ok(None)
    }
    pub fn next(&mut self) -> Result<Option<Frame<'_>>> {
        ensure!(!self.failed, "connection failed");
        if self.active.is_some() {
            return Ok(None);
        }
        if self.metadata_first {
            if let Some(header) = self.metadata()? {
                return Ok(Some(Frame { header, bytes: &[] }));
            }
        }
        let credits = std::array::from_fn(|i| self.flows[i].available());
        let selected = self
            .queue
            .next(&credits)?
            .map(|slice| (slice.channel, slice.bytes.len()));
        if let Some((channel, length)) = selected {
            let header = self.flows[channel as usize - 1].send(Kind::Data, length as u32, 0)?;
            self.active = Some(header);
            self.metadata_first = true;
            return Ok(Some(Frame {
                header,
                bytes: self.queue.reserved().expect("reserved slice").bytes,
            }));
        }
        Ok(self.metadata()?.map(|header| Frame { header, bytes: &[] }))
    }
    /// Reborrow the current writer reservation after a nonblocking write call
    /// returns. Never finish/release its bytes while native code retains a borrow.
    /// This permits a single owner to service inbound events between partial
    /// writes without copying an outbound payload into a second staging buffer.
    pub fn reserved(&self) -> Option<Frame<'_>> {
        let header = self.active?;
        let bytes = if header.kind == Kind::Data {
            self.queue.reserved().expect("reserved slice").bytes
        } else {
            &[]
        };
        Some(Frame { header, bytes })
    }
    /// Finish only when native writer has retired its borrow. False retires the
    /// entire physical connection, even when no bytes were observed locally.
    pub fn finish(&mut self, success: bool) -> Result<()> {
        let header = self
            .active
            .take()
            .ok_or_else(|| anyhow::anyhow!("no reserved frame"))?;
        if header.kind == Kind::Data {
            self.queue.finish(success)?;
        }
        if self.closing[header.channel as usize - 1] {
            self.queue.clear(header.channel)?;
        }
        self.failed |= !success;
        Ok(())
    }
    /// Adapter event after both local bridge I/O directions actually stop and
    /// release their byte buffers. A cancellation request alone is insufficient.
    pub fn consumer_retired(&mut self, channel: Channel, generation: u64) -> Result<()> {
        let index = channel as usize - 1;
        ensure!(
            !self.failed && self.closing[index] && self.generations[index] == generation,
            "stale consumer retirement"
        );
        self.consumer_retired[index] = true;
        Ok(())
    }
    /// New generation must be agreed over authenticated management. Old peer
    /// bodies must be fully drained before this single-owner operation executes.
    pub fn replace_channel(&mut self, channel: Channel, generation: u64) -> Result<()> {
        let index = channel as usize - 1;
        ensure!(
            self.retired(channel) && generation > self.generations[index],
            "channel still owned or stale generation"
        );
        let flow = Flow::new(self.session, generation, channel, self.capacity)?;
        self.flows[index] = flow;
        self.generations[index] = generation;
        self.consumer_retired[index] = false;
        self.closing[index] = false;
        self.enabled[index] = false;
        self.credit[index] = 0;
        Ok(())
    }
    pub fn retired(&self, channel: Channel) -> bool {
        let index = channel as usize - 1;
        !self.failed
            && self.consumer_retired[index]
            && self.flows[index].retired()
            && self.terminal[index].is_none()
            && self.active.is_none_or(|header| header.channel != channel)
    }
}
