use super::*;
use anyhow::Context;

pub(super) struct Header {
    pub operation: u8,
    pub key: Option<Key>,
    pub body: usize,
}
impl Header {
    pub fn decode(input: &[u8]) -> Result<Self> {
        ensure!(
            input.len() == HEADER_BYTES,
            "invalid management header length"
        );
        ensure!(
            &input[..5] == b"BLMG\x01" && input[7] == 0,
            "invalid management prefix"
        );
        let operation = input[5];
        let key = Self::key(operation, input)?;
        let body = u32::from_be_bytes(input[24..28].try_into()?) as usize;
        ensure!(
            body == body_length(operation, key.map(|key| key.resource))?,
            "invalid management body length"
        );
        Ok(Self {
            operation,
            key,
            body,
        })
    }
    fn key(operation: u8, input: &[u8]) -> Result<Option<Key>> {
        let request = u64::from_be_bytes(input[8..16].try_into()?);
        let generation = u64::from_be_bytes(input[16..24].try_into()?);
        if operation == 0 {
            ensure!(
                input[6] == 0 && request == 0 && generation == 0,
                "capabilities has resource identity"
            );
            return Ok(None);
        }
        let key = Key {
            resource: Resource::try_from(input[6])?,
            request,
            generation,
        };
        key.validate()?;
        Ok(Some(key))
    }
    fn encode(&self, out: &mut [u8]) {
        out[..HEADER_BYTES].fill(0);
        out[..5].copy_from_slice(b"BLMG\x01");
        out[5] = self.operation;
        if let Some(key) = self.key {
            out[6] = key.resource as u8;
            out[8..16].copy_from_slice(&key.request.to_be_bytes());
            out[16..24].copy_from_slice(&key.generation.to_be_bytes());
        }
        out[24..28].copy_from_slice(&(self.body as u32).to_be_bytes());
    }
}
fn offer_length(resource: Resource) -> usize {
    match resource {
        Resource::DisplaySession => 32,
        Resource::Camera => 45,
        Resource::Microphone | Resource::Speakers => 36,
    }
}
fn body_length(operation: u8, resource: Option<Resource>) -> Result<usize> {
    Ok(match operation {
        0 | 4 | 5 => 1,
        1 => offer_length(resource.context("missing resource")?),
        2 | 3 => 0,
        _ => anyhow::bail!("unknown management operation"),
    })
}
impl Message {
    fn header(&self) -> Result<Header> {
        let (operation, key) = match self {
            Self::Capabilities(mask) => {
                ensure!(*mask <= 15, "invalid capability mask");
                (0, None)
            }
            Self::Offer(key, _, profile) => {
                profile.validate(key.resource)?;
                (1, Some(*key))
            }
            Self::Prepared(key) => (2, Some(*key)),
            Self::Commit(key) => (3, Some(*key)),
            Self::Result(key, _) => (4, Some(*key)),
            Self::Revoke(key, _) => (5, Some(*key)),
        };
        if let Some(key) = key {
            key.validate()?;
        }
        Ok(Header {
            operation,
            key,
            body: body_length(operation, key.map(|key| key.resource))?,
        })
    }
    /// Validate the entire model and output extent before changing caller bytes.
    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        let header = self.header()?;
        let length = HEADER_BYTES + header.body;
        ensure!(out.len() >= length, "short management output");
        header.encode(out);
        let body = &mut out[HEADER_BYTES..length];
        match self {
            Self::Capabilities(mask) => body[0] = *mask,
            Self::Offer(_, credential, profile) => {
                body[..32].copy_from_slice(credential.as_bytes());
                profile.encode(&mut body[32..]);
            }
            Self::Prepared(_) | Self::Commit(_) => {}
            Self::Result(_, value) => body[0] = *value as u8,
            Self::Revoke(_, value) => body[0] = *value as u8,
        }
        Ok(length)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        ensure!(input.len() >= HEADER_BYTES, "truncated management header");
        let header = Header::decode(&input[..HEADER_BYTES])?;
        ensure!(
            input.len() == HEADER_BYTES + header.body,
            "invalid management record extent"
        );
        let body = &input[HEADER_BYTES..];
        if header.operation == 0 {
            ensure!(body[0] <= 15, "invalid capability mask");
            return Ok(Self::Capabilities(body[0]));
        }
        Self::decode_keyed(header, body)
    }
    fn decode_keyed(header: Header, body: &[u8]) -> Result<Self> {
        let key = header.key.context("missing resource")?;
        Ok(match header.operation {
            1 => Self::Offer(
                key,
                Credential::new(body[..32].try_into()?),
                Profile::decode(key.resource, &body[32..])?,
            ),
            2 => Self::Prepared(key),
            3 => Self::Commit(key),
            4 => Self::Result(key, Outcome::try_from(body[0])?),
            5 => Self::Revoke(key, Reason::try_from(body[0])?),
            _ => anyhow::bail!("invalid keyed operation"),
        })
    }
}
impl Profile {
    fn encode(&self, out: &mut [u8]) {
        match self {
            Self::Display => {}
            Self::Camera(value) => value.encode(out),
            Self::Audio(value) => {
                out[0] = value.processing;
                out[1..3].copy_from_slice(&value.buffer_ms.to_be_bytes());
                out[3] = u8::from(value.background);
            }
        }
    }
    fn decode(resource: Resource, input: &[u8]) -> Result<Self> {
        let profile = match resource {
            Resource::DisplaySession => Self::Display,
            Resource::Camera => Self::Camera(Camera::decode(input)?),
            Resource::Microphone | Resource::Speakers => {
                ensure!(input[3] <= 1, "invalid background flag");
                Self::Audio(Audio {
                    processing: input[0],
                    buffer_ms: u16::from_be_bytes(input[1..3].try_into()?),
                    background: input[3] != 0,
                })
            }
        };
        profile.validate(resource)?;
        Ok(profile)
    }
}
impl Camera {
    fn encode(&self, out: &mut [u8]) {
        for (offset, value) in [
            (0, self.width),
            (2, self.height),
            (5, self.bitrate),
            (7, self.min_bitrate),
            (9, self.freshness_ms),
        ] {
            out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
        }
        out[4] = self.fps;
        out[11] = self.lens;
        out[12] = u8::from(self.background) | (u8::from(self.adaptive) << 1);
    }
    fn decode(input: &[u8]) -> Result<Self> {
        ensure!(input[12] <= 3, "invalid camera flags");
        Ok(Self {
            width: u16::from_be_bytes(input[0..2].try_into()?),
            height: u16::from_be_bytes(input[2..4].try_into()?),
            fps: input[4],
            bitrate: u16::from_be_bytes(input[5..7].try_into()?),
            min_bitrate: u16::from_be_bytes(input[7..9].try_into()?),
            freshness_ms: u16::from_be_bytes(input[9..11].try_into()?),
            lens: input[11],
            background: input[12] & 1 != 0,
            adaptive: input[12] & 2 != 0,
        })
    }
}
