use super::*;
use crate::connection::wire::MAX_COUNTER;
use anyhow::{ensure, Result};

const MAX_BODY: usize = 128;
fn counter(value: u64) -> Result<()> {
    ensure!(
        (1..=MAX_COUNTER).contains(&value),
        "invalid broker control counter"
    );
    Ok(())
}
fn push_counter(out: &mut Vec<u8>, value: u64) -> Result<()> {
    counter(value)?;
    out.extend_from_slice(&value.to_be_bytes());
    Ok(())
}
fn push_ticket(out: &mut Vec<u8>, ticket: &Ticket) -> Result<()> {
    push_counter(out, ticket.epoch)?;
    out.extend_from_slice(ticket.peer.as_str().as_bytes());
    Ok(())
}
fn push_optional(out: &mut Vec<u8>, ticket: &Option<Ticket>) -> Result<()> {
    out.push(u8::from(ticket.is_some()));
    if let Some(ticket) = ticket {
        push_ticket(out, ticket)?;
    }
    Ok(())
}
struct Input<'a>(&'a [u8]);
impl<'a> Input<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= MAX_BODY,
            "invalid control extent"
        );
        Ok(Self(bytes))
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        ensure!(count <= self.0.len(), "truncated control record");
        let (value, rest) = self.0.split_at(count);
        self.0 = rest;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn flag(&mut self) -> Result<bool> {
        let value = self.byte()?;
        ensure!(value <= 1, "invalid control boolean");
        Ok(value == 1)
    }
    fn counter(&mut self) -> Result<u64> {
        let value = u64::from_be_bytes(self.take(8)?.try_into()?);
        counter(value)?;
        Ok(value)
    }
    fn peer(&mut self) -> Result<PeerId> {
        PeerId::parse(std::str::from_utf8(self.take(64)?)?.to_owned())
    }
    fn ticket(&mut self) -> Result<Ticket> {
        Ok(Ticket {
            epoch: self.counter()?,
            peer: self.peer()?,
        })
    }
    fn optional(&mut self) -> Result<Option<Ticket>> {
        if self.flag()? {
            Ok(Some(self.ticket()?))
        } else {
            Ok(None)
        }
    }
    fn short(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into()?))
    }
    fn finish(self) -> Result<()> {
        ensure!(self.0.is_empty(), "trailing control bytes");
        Ok(())
    }
}

impl Command {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(MAX_BODY);
        match self {
            Self::Inspect => out.push(0),
            Self::Connect { revision, target } => {
                out.push(1);
                push_counter(&mut out, *revision)?;
                push_counter(&mut out, *target)?;
            }
            Self::Decide {
                connection,
                ticket,
                allow,
                remember,
            } => {
                ensure!(*allow || !remember, "cannot remember a denial");
                out.push(2);
                push_counter(&mut out, *connection)?;
                push_ticket(&mut out, ticket)?;
                out.extend_from_slice(&[u8::from(*allow), u8::from(*remember)]);
            }
            Self::Disconnect { connection, ticket } => {
                out.push(3);
                push_counter(&mut out, *connection)?;
                push_optional(&mut out, ticket)?;
            }
            Self::Revoke { peer } => {
                out.push(4);
                out.extend_from_slice(peer.as_str().as_bytes());
            }
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut input = Input::new(bytes)?;
        let result = match input.byte()? {
            0 => Self::Inspect,
            1 => Self::Connect {
                revision: input.counter()?,
                target: input.counter()?,
            },
            2 => Self::Decide {
                connection: input.counter()?,
                ticket: input.ticket()?,
                allow: input.flag()?,
                remember: input.flag()?,
            },
            3 => Self::Disconnect {
                connection: input.counter()?,
                ticket: input.optional()?,
            },
            4 => Self::Revoke {
                peer: input.peer()?,
            },
            _ => anyhow::bail!("unknown broker control command"),
        };
        input.finish()?;
        result.encode()?;
        Ok(result)
    }
}

impl State {
    fn parse(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::Discovering,
            1 => Self::Pending,
            2 => Self::Approved,
            3 => Self::Authenticated,
            4 => Self::Closing,
            5 => Self::Quarantined,
            _ => anyhow::bail!("invalid broker link state"),
        })
    }
    fn validate(self, ticket: &Option<Ticket>, trusted: bool) -> Result<()> {
        if matches!(self, Self::Pending | Self::Approved | Self::Authenticated) {
            ensure!(ticket.is_some(), "link state requires pairing ticket");
        }
        if self == Self::Discovering {
            ensure!(ticket.is_none(), "discovery has no pairing ticket");
        }
        ensure!(
            !trusted || ticket.is_some(),
            "saved trust requires known peer"
        );
        Ok(())
    }
}
impl Notice {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(MAX_BODY);
        match self {
            Self::Begin { revision, local } => {
                out.push(0);
                push_counter(&mut out, *revision)?;
                out.extend_from_slice(local.as_str().as_bytes());
            }
            Self::Device {
                revision,
                target,
                vendor,
                product,
                accessory,
            } => {
                out.push(1);
                push_counter(&mut out, *revision)?;
                push_counter(&mut out, *target)?;
                out.extend_from_slice(&vendor.to_be_bytes());
                out.extend_from_slice(&product.to_be_bytes());
                out.push(u8::from(*accessory));
            }
            Self::Link {
                revision,
                connection,
                ticket,
                state,
                trusted,
            } => {
                state.validate(ticket, *trusted)?;
                out.push(2);
                push_counter(&mut out, *revision)?;
                push_counter(&mut out, *connection)?;
                out.extend_from_slice(&[*state as u8, u8::from(*trusted)]);
                push_optional(&mut out, ticket)?;
            }
            Self::Saved { revision, peer } => {
                out.push(3);
                push_counter(&mut out, *revision)?;
                out.extend_from_slice(peer.as_str().as_bytes());
            }
            Self::End { revision } => {
                out.push(4);
                push_counter(&mut out, *revision)?;
            }
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut input = Input::new(bytes)?;
        let operation = input.byte()?;
        let revision = input.counter()?;
        let result = match operation {
            0 => Self::Begin {
                revision,
                local: input.peer()?,
            },
            1 => Self::Device {
                revision,
                target: input.counter()?,
                vendor: input.short()?,
                product: input.short()?,
                accessory: input.flag()?,
            },
            2 => decode_link(&mut input, revision)?,
            3 => Self::Saved {
                revision,
                peer: input.peer()?,
            },
            4 => Self::End { revision },
            _ => anyhow::bail!("unknown broker control notice"),
        };
        input.finish()?;
        result.encode()?;
        Ok(result)
    }
}
fn decode_link(input: &mut Input<'_>, revision: u64) -> Result<Notice> {
    let connection = input.counter()?;
    let state = State::parse(input.byte()?)?;
    let trusted = input.flag()?;
    let ticket = input.optional()?;
    Ok(Notice::Link {
        revision,
        connection,
        ticket,
        state,
        trusted,
    })
}
