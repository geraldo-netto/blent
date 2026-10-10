//! Per-direction credit/sequence accounting. No allocation or native I/O.
use super::wire::{Channel, Header, Kind, MAX_CREDIT};
use anyhow::{ensure, Result};

/// Created only after authenticated control agrees a session, channel generation
/// and receive window. Header IDs are not credentials. Serialize access through
/// the connection owner. Reserve writer capacity before `send`; a later I/O
/// failure retires the stream rather than refunding possibly transmitted bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flow {
    session: u64,
    generation: u64,
    channel: Channel,
    capacity: u32,
    send_credit: u32,
    receive_credit: u32,
    received: u32,
    send_sequence: u64,
    receive_sequence: u64,
    closed: bool,
    local_terminal: bool,
    peer_terminal: bool,
}
impl Flow {
    pub fn available(&self) -> u32 {
        if self.closed || self.send_sequence > super::wire::MAX_COUNTER {
            0
        } else {
            self.send_credit
        }
    }
    pub fn new(session: u64, generation: u64, channel: Channel, capacity: u32) -> Result<Self> {
        ensure!(
            (1..=MAX_CREDIT).contains(&capacity),
            "invalid channel capacity"
        );
        Header {
            session,
            generation,
            channel,
            sequence: 0,
            kind: Kind::Close,
            length: 0,
            credit: 0,
        }
        .validate()?;
        Ok(Self {
            session,
            generation,
            channel,
            capacity,
            send_credit: 0,
            receive_credit: 0,
            received: 0,
            send_sequence: 0,
            receive_sequence: 0,
            closed: false,
            local_terminal: false,
            peer_terminal: false,
        })
    }
    pub fn send(&mut self, kind: Kind, length: u32, credit: u32) -> Result<Header> {
        ensure!(!self.closed, "channel closed");
        let header = Header {
            session: self.session,
            generation: self.generation,
            channel: self.channel,
            sequence: self.send_sequence,
            kind,
            length,
            credit,
        };
        header.validate()?;
        self.apply_send(header)?;
        self.send_sequence += 1;
        Ok(header)
    }
    fn apply_send(&mut self, header: Header) -> Result<()> {
        match header.kind {
            Kind::Data => {
                ensure!(header.length <= self.send_credit, "peer credit exhausted");
                self.send_credit -= header.length;
            }
            Kind::Credit => {
                ensure!(
                    header.credit <= self.capacity - self.receive_credit - self.received,
                    "receive credit overflow"
                );
                self.receive_credit += header.credit;
            }
            Kind::Close | Kind::Reset => {
                self.closed = true;
                self.local_terminal = true;
            }
        }
        Ok(())
    }
    /// Validate identity, sequence and bounds before committing any state. A data
    /// header reserves its complete body; truncated bodies retire the stream.
    pub fn receive(&mut self, header: Header) -> Result<()> {
        ensure!(!self.closed, "channel closed");
        self.validate_receive(header)?;
        self.apply_receive(header)?;
        self.receive_sequence += 1;
        Ok(())
    }
    fn validate_receive(&self, header: Header) -> Result<()> {
        header.validate()?;
        ensure!(
            header.session == self.session
                && header.generation == self.generation
                && header.channel == self.channel,
            "stale or misrouted channel"
        );
        ensure!(
            header.sequence == self.receive_sequence,
            "invalid receive sequence"
        );
        Ok(())
    }
    /// Terminal messages revoke local authority immediately. An ordered peer
    /// acknowledgement proves all earlier peer frames have been drained.
    pub fn terminal(&mut self, kind: Kind) -> Result<Header> {
        ensure!(
            matches!(kind, Kind::Close | Kind::Reset),
            "not a terminal kind"
        );
        ensure!(!self.local_terminal, "terminal already sent");
        let header = Header {
            session: self.session,
            generation: self.generation,
            channel: self.channel,
            sequence: self.send_sequence,
            kind,
            length: 0,
            credit: 0,
        };
        header.validate()?;
        self.apply_send(header)?;
        self.send_sequence += 1;
        Ok(header)
    }
    /// Validate and discard already-authorized in-flight traffic after local
    /// retirement. No new credit is granted. Bodies still need bounded draining.
    pub fn drain(&mut self, header: Header) -> Result<()> {
        ensure!(
            self.closed && !self.peer_terminal,
            "not awaiting peer retirement"
        );
        self.validate_receive(header)?;
        self.apply_receive(header)?;
        if header.kind == Kind::Data {
            self.received -= header.length;
        }
        self.receive_sequence += 1;
        Ok(())
    }
    pub fn retired(&self) -> bool {
        self.local_terminal && self.peer_terminal
    }
    fn apply_receive(&mut self, header: Header) -> Result<()> {
        match header.kind {
            Kind::Data => {
                ensure!(header.length <= self.receive_credit, "ungranted peer data");
                self.receive_credit -= header.length;
                self.received += header.length;
            }
            Kind::Credit => {
                ensure!(
                    header.credit <= self.capacity - self.send_credit,
                    "send credit overflow"
                );
                self.send_credit += header.credit;
            }
            Kind::Close | Kind::Reset => {
                self.closed = true;
                self.peer_terminal = true;
            }
        }
        Ok(())
    }
    /// Return receive capacity only after the consumer releases these bytes.
    pub fn release(&mut self, bytes: u32) -> Result<()> {
        ensure!(!self.closed, "channel closed");
        ensure!(
            bytes > 0 && bytes <= self.received,
            "invalid receive release"
        );
        self.received -= bytes;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t846_sequences_never_wrap() {
        let mut flow = Flow::new(1, 1, Channel::Control, 16).unwrap();
        flow.send_sequence = super::super::wire::MAX_COUNTER;
        let header = flow.send(Kind::Credit, 0, 1).unwrap();
        assert_eq!(header.sequence, super::super::wire::MAX_COUNTER);
        let before = flow.clone();
        assert!(flow.send(Kind::Credit, 0, 1).is_err());
        assert_eq!(flow, before);
        flow.receive_sequence = super::super::wire::MAX_COUNTER;
        flow.receive(header).unwrap();
        let before = flow.clone();
        assert!(flow.receive(header).is_err());
        assert_eq!(flow, before);
    }
}
