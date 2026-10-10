//! Compose bounded public identity discovery and stock TLS over any owned byte
//! transport. UI/approval and native device access remain separate adapters.
use super::{Resource, Stream};
use crate::secure_connection::{self, Identity, PeerCertificate};
use anyhow::{ensure, Result};
use blent_config::connection::{
    hello::{Hello, HEADER_BYTES},
    pairing::{PeerId, Ticket},
    ready,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Candidate {
    resource: Resource,
    certificate: PeerCertificate,
    peer: PeerId,
}
pub struct Verified {
    pub resource: Resource,
    pub ticket: Ticket,
    pub peer: PeerId,
}
impl Candidate {
    pub fn peer(&self) -> &PeerId {
        &self.peer
    }
    /// Caller starts this only after approval. The result is TLS proof, not a
    /// media lease: Pairing::authenticated must accept its ticket/proven peer
    /// after the await, rejecting intervening revocation, timeout or replacement.
    pub async fn authenticate(self, identity: &Identity, ticket: Ticket) -> Result<Verified> {
        ensure!(
            ticket.peer == self.peer,
            "approval names a different identity"
        );
        let Resource { mut stream, owner } = self.resource;
        // Human approval can take longer than TLS's ten-second setup budget.
        // This untrusted marker coordinates scheduling, never peer authority.
        tokio::time::timeout(Duration::from_secs(120), synchronize(&mut *stream)).await??;
        let stream = secure_connection::accept(stream, identity, &self.certificate).await?;
        Ok(Verified {
            resource: Resource {
                stream: Box::new(stream),
                owner,
            },
            ticket,
            peer: self.peer,
        })
    }
    /// Denial must observe actual native retirement; Drop alone only requests it.
    pub async fn reject(self) -> Result<()> {
        let Resource { stream, owner } = self.resource;
        drop(stream);
        owner.retire().await
    }
}
async fn synchronize(stream: &mut dyn Stream) -> Result<()> {
    stream.write_all(&ready::encode(true)).await?;
    stream.flush().await?;
    let mut bytes = [0; ready::BYTES];
    stream.read_exact(&mut bytes).await?;
    ready::decode(&bytes, false)
}
/// Public certificate exchange has an absolute deadline and a fixed allocation
/// bound. No peer-controlled length reaches allocation before header validation.
pub async fn discover(
    mut resource: Resource,
    identity: &Identity,
    timeout: Duration,
) -> Result<Candidate> {
    ensure!(
        !timeout.is_zero() && timeout <= Duration::from_secs(60),
        "invalid discovery timeout"
    );
    let certificate = tokio::time::timeout(
        timeout,
        exchange(&mut *resource.stream, identity.certificate()),
    )
    .await??;
    let fingerprint = certificate.fingerprint();
    let peer = PeerId::parse(hex(&fingerprint))?;
    Ok(Candidate {
        resource,
        certificate,
        peer,
    })
}
fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for value in bytes {
        text.push(DIGITS[(value >> 4) as usize] as char);
        text.push(DIGITS[(value & 15) as usize] as char);
    }
    text
}
async fn exchange(stream: &mut dyn Stream, certificate: &[u8]) -> Result<PeerCertificate> {
    let header = Hello {
        host: true,
        certificate_bytes: certificate.len().try_into()?,
    }
    .encode()?;
    stream.write_all(&header).await?;
    stream.write_all(certificate).await?;
    stream.flush().await?;
    let mut header = [0; HEADER_BYTES];
    stream.read_exact(&mut header).await?;
    let length = Hello::decode(&header, false)?.certificate_bytes as usize;
    let mut certificate = vec![0; length];
    stream.read_exact(&mut certificate).await?;
    PeerCertificate::parse(certificate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessory::Owner;
    use crate::secure_connection::IdentityRecord;
    use blent_config::{
        connection::pairing::Pairing,
        retirement::{Admission, Completion, Pool},
    };
    use std::{future::Future, pin::Pin};
    struct MemoryOwner(Admission);
    impl Owner for MemoryOwner {
        fn completion(&self) -> Completion {
            self.0.completion()
        }
        fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
            Box::pin(async move {
                drop(self);
                Ok(())
            })
        }
    }
    fn resources() -> (Resource, tokio::io::DuplexStream, Completion) {
        let (stream, remote) = tokio::io::duplex(8192);
        let owner = Box::new(MemoryOwner(Pool::new(1).unwrap().reserve().unwrap()));
        let completion = owner.completion();
        (
            Resource {
                stream: Box::new(stream),
                owner,
            },
            remote,
            completion,
        )
    }
    fn identity() -> Identity {
        Identity::load(IdentityRecord::generate().unwrap()).unwrap()
    }
    async fn remote_hello(remote: &mut tokio::io::DuplexStream, certificate: &[u8]) {
        let mut header = [0; HEADER_BYTES];
        remote.read_exact(&mut header).await.unwrap();
        let length = Hello::decode(&header, true).unwrap().certificate_bytes;
        let mut host = vec![0; length as usize];
        remote.read_exact(&mut host).await.unwrap();
        remote
            .write_all(
                &Hello {
                    host: false,
                    certificate_bytes: certificate.len() as u16,
                }
                .encode()
                .unwrap(),
            )
            .await
            .unwrap();
        remote.write_all(certificate).await.unwrap();
    }
    #[tokio::test]
    async fn t846_candidate_approval_tls_and_revocation_are_separate_boundaries() {
        let (resource, mut remote, completion) = resources();
        let host = identity();
        let tablet = identity();
        let (candidate, _) = tokio::join!(
            discover(resource, &host, Duration::from_secs(1)),
            remote_hello(&mut remote, tablet.certificate())
        );
        let candidate = candidate.unwrap();
        let mut pairing = Pairing::default();
        let ticket = pairing
            .begin(candidate.peer().clone(), false, 0, 1000)
            .unwrap();
        assert!(pairing.authenticated(&ticket, candidate.peer(), 1).is_err());
        pairing.approve(&ticket, true, 1).unwrap();
        let approved_host = PeerCertificate::parse(host.certificate().to_vec()).unwrap();
        let (verified, client) =
            tokio::join!(candidate.authenticate(&host, ticket.clone()), async {
                let mut marker = [0; ready::BYTES];
                remote.read_exact(&mut marker).await?;
                ready::decode(&marker, true)?;
                remote.write_all(&ready::encode(false)).await?;
                secure_connection::connect(remote, &tablet, &approved_host).await
            });
        let verified = verified.unwrap();
        let mut client = client.unwrap();
        // Cancellation arriving during TLS prevents installation of the proven stream.
        assert!(pairing.cancel(&ticket));
        assert!(pairing
            .authenticated(&verified.ticket, &verified.peer, 2)
            .is_err());
        let Resource { mut stream, owner } = verified.resource;
        client.write_all(b"tls").await.unwrap();
        client.flush().await.unwrap();
        let mut bytes = [0; 3];
        stream.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"tls");
        drop(stream);
        drop(client);
        owner.retire().await.unwrap();
        assert!(completion.is_complete());
        pairing.retired(&ticket).unwrap();
    }
    #[tokio::test(start_paused = true)]
    async fn t846_human_approval_skew_does_not_consume_tls_handshake_deadline() {
        let (resource, mut remote, completion) = resources();
        let host = identity();
        let tablet = identity();
        let (candidate, _) = tokio::join!(
            discover(resource, &host, Duration::from_secs(1)),
            remote_hello(&mut remote, tablet.certificate())
        );
        let candidate = candidate.unwrap();
        let ticket = Ticket {
            epoch: 1,
            peer: candidate.peer().clone(),
        };
        let approved_host = PeerCertificate::parse(host.certificate().to_vec()).unwrap();
        let delayed_peer = async {
            tokio::time::sleep(Duration::from_secs(12)).await;
            let mut ready = [0; 8];
            remote.read_exact(&mut ready).await?;
            ensure!(&ready == b"BLPR\x01\x01\0\0", "wrong host readiness");
            remote.write_all(b"BLPR\x01\0\0\0").await?;
            secure_connection::connect(remote, &tablet, &approved_host).await
        };
        let (verified, peer) = tokio::join!(candidate.authenticate(&host, ticket), delayed_peer);
        assert!(
            verified.is_ok(),
            "T846 human approval exhausted TLS deadline: {:?}",
            verified.as_ref().err()
        );
        let verified = verified.unwrap();
        drop(peer.unwrap());
        drop(verified.resource.stream);
        verified.resource.owner.retire().await.unwrap();
        assert!(completion.is_complete());
    }
    #[tokio::test(start_paused = true)]
    async fn t846_readiness_silence_invalid_roles_and_truncation_retire_owner() {
        let host = identity();
        let tablet = identity();
        for marker in [
            None,
            Some(b"BLPR\x01\x01\0\0".to_vec()),
            Some(b"bad".to_vec()),
        ] {
            let (resource, mut remote, completion) = resources();
            let (candidate, _) = tokio::join!(
                discover(resource, &host, Duration::from_secs(1)),
                remote_hello(&mut remote, tablet.certificate())
            );
            let candidate = candidate.unwrap();
            let ticket = Ticket {
                epoch: 1,
                peer: candidate.peer().clone(),
            };
            if let Some(bytes) = marker {
                remote.write_all(&bytes).await.unwrap();
                remote.shutdown().await.unwrap();
            }
            assert!(candidate.authenticate(&host, ticket).await.is_err());
            assert!(completion.is_complete());
        }
    }
    #[tokio::test]
    async fn t846_candidate_denial_or_wrong_ticket_retires_owner() {
        for deny in [true, false] {
            let (resource, mut remote, completion) = resources();
            let host = identity();
            let tablet = identity();
            let (candidate, _) = tokio::join!(
                discover(resource, &host, Duration::from_secs(1)),
                remote_hello(&mut remote, tablet.certificate())
            );
            let candidate = candidate.unwrap();
            if deny {
                candidate.reject().await.unwrap();
            } else {
                let ticket = Ticket {
                    epoch: 1,
                    peer: PeerId::parse("0".repeat(64)).unwrap(),
                };
                assert!(candidate.authenticate(&host, ticket).await.is_err());
            }
            assert!(completion.is_complete());
        }
    }
    #[tokio::test(start_paused = true)]
    async fn t846_discovery_deadline_invalid_lengths_and_truncation_are_bounded() {
        let host = identity();
        for timeout in [Duration::ZERO, Duration::from_secs(61)] {
            let (resource, _remote, completion) = resources();
            assert!(discover(resource, &host, timeout).await.is_err());
            assert!(completion.is_complete());
        }
        let (resource, _silent, completion) = resources();
        assert!(discover(resource, &host, Duration::from_secs(1))
            .await
            .is_err());
        assert!(completion.is_complete());
        for input in [
            b"bad".to_vec(),
            b"BLPA\x01\x00\xff\xff".to_vec(),
            b"BLPA\x01\x00\x00\x01x".to_vec(),
            b"BLPA\x01\x00\x00\x02x".to_vec(),
        ] {
            let (resource, mut remote, completion) = resources();
            remote.write_all(&input).await.unwrap();
            remote.shutdown().await.unwrap();
            assert!(discover(resource, &host, Duration::from_secs(1))
                .await
                .is_err());
            assert!(completion.is_complete());
        }
    }
}
