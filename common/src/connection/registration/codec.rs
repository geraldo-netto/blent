use super::*;
const PREFIX: &[u8; 5] = b"BLRG\x01";

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>> {
        counter(self.request())?;
        let (operation, body) = self.body()?;
        let mut packet = vec![0; HEADER_BYTES];
        packet[..5].copy_from_slice(PREFIX);
        packet[5] = operation;
        packet[8..16].copy_from_slice(&self.request().to_be_bytes());
        packet.extend_from_slice(&body);
        Ok(packet)
    }
    fn body(&self) -> Result<(u8, Vec<u8>)> {
        Ok(match self {
            Self::Control { command, .. } => (5, command.encode()?),
            Self::Report { notice, .. } => (6, notice.encode()?),
            Self::Register {
                peer,
                resource,
                credential,
                profile,
                ..
            } => (
                0,
                encode_registration(peer, *resource, credential, profile)?,
            ),
            Self::Unregister { route, .. } => (1, encode_route(*route)?.to_vec()),
            Self::Registered { route, .. } => (2, encode_route(*route)?.to_vec()),
            Self::Status { route, state, .. } => {
                let mut body = encode_route(*route)?.to_vec();
                body.push(*state as u8);
                (3, body)
            }
            Self::Rejected { reason, .. } => (4, vec![*reason as u8]),
        })
    }
    pub fn decode(packet: &[u8]) -> Result<Self> {
        ensure!(
            (HEADER_BYTES..=MAX_PACKET).contains(&packet.len()),
            "invalid registration packet size"
        );
        ensure!(
            &packet[..5] == PREFIX && packet[6..8] == [0, 0],
            "invalid registration prefix"
        );
        let request = u64::from_be_bytes(packet[8..16].try_into()?);
        counter(request)?;
        decode_body(packet[5], request, &packet[HEADER_BYTES..])
    }
}
fn decode_body(operation: u8, request: u64, body: &[u8]) -> Result<Message> {
    Ok(match operation {
        5 => Message::Control {
            request,
            command: super::super::control::Command::decode(body)?,
        },
        6 => Message::Report {
            request,
            notice: super::super::control::Notice::decode(body)?,
        },
        0 => decode_registration(request, body)?,
        1 => Message::Unregister {
            request,
            route: decode_route(body)?,
        },
        2 => Message::Registered {
            request,
            route: decode_route(body)?,
        },
        3 => {
            ensure!(body.len() == 17, "invalid registration status extent");
            Message::Status {
                request,
                route: decode_route(&body[..16])?,
                state: State::try_from(body[16])?,
            }
        }
        4 => {
            ensure!(body.len() == 1, "invalid registration rejection extent");
            Message::Rejected {
                request,
                reason: Rejection::try_from(body[0])?,
            }
        }
        _ => anyhow::bail!("unknown local registration operation"),
    })
}
fn encode_registration(
    peer: &PeerId,
    resource: Resource,
    credential: &Credential,
    profile: &Profile,
) -> Result<Vec<u8>> {
    let mut encoded_profile = [0; super::super::management::MAX_RECORD];
    let length = profile.encode_for(resource, &mut encoded_profile)?;
    let mut body = Vec::with_capacity(97 + length);
    body.extend_from_slice(peer.as_str().as_bytes());
    body.push(resource as u8);
    body.extend_from_slice(credential.as_bytes());
    body.extend_from_slice(&encoded_profile[..length]);
    Ok(body)
}
fn decode_registration(request: u64, body: &[u8]) -> Result<Message> {
    ensure!(body.len() >= 97, "truncated local registration");
    let peer = PeerId::parse(std::str::from_utf8(&body[..64])?.to_owned())?;
    let resource = Resource::try_from(body[64])?;
    let credential = Credential::new(body[65..97].try_into()?);
    let profile = Profile::decode_for(resource, &body[97..])?;
    Ok(Message::Register {
        request,
        peer,
        resource,
        credential,
        profile,
    })
}
fn encode_route(route: RouteId) -> Result<[u8; 16]> {
    counter(route.owner)?;
    counter(route.revision)?;
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&route.owner.to_be_bytes());
    bytes[8..].copy_from_slice(&route.revision.to_be_bytes());
    Ok(bytes)
}
fn decode_route(body: &[u8]) -> Result<RouteId> {
    ensure!(body.len() == 16, "invalid local route extent");
    let route = RouteId {
        owner: u64::from_be_bytes(body[..8].try_into()?),
        revision: u64::from_be_bytes(body[8..].try_into()?),
    };
    counter(route.owner)?;
    counter(route.revision)?;
    Ok(route)
}
