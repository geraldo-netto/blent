//! T846 standard TLS adapter. No native USB access or implicit trust-on-first-use.
//! Callers supply an explicitly approved exact peer certificate; only public
//! identities may cross the physical link before approval and TLS authentication.
use anyhow::{ensure, Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::sign::{CertifiedKey, SingleCertAndKey};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub mod identity_store;
mod retained_identity;
pub use retained_identity::RetainedIdentity;
#[cfg(target_os = "linux")]
mod private_directory;
#[cfg(target_os = "linux")]
pub mod trust_store;

pub const MAX_CERTIFICATE: usize = 4096;
pub const MAX_PRIVATE_KEY: usize = 4096;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const ALPN: &[u8] = b"blent-usb/1";
const PREFACE: &[u8; 10] = b"BLENTUSB\0\x01";

/// Storage adapter owns restrictive permissions and atomic persistence. Never
/// derive Debug or log this record: private key bytes are secret.
pub struct IdentityRecord {
    pub certificate: Vec<u8>,
    pub private_key: Vec<u8>,
}
impl IdentityRecord {
    pub fn generate() -> Result<Self> {
        let generated = rcgen::generate_simple_self_signed(vec!["blent.local".into()])?;
        Ok(Self {
            certificate: generated.cert.der().to_vec(),
            private_key: generated.signing_key.serialize_der(),
        })
    }
}
pub struct Identity {
    certificate: CertificateDer<'static>,
    certified: CertifiedKey,
}
impl Identity {
    pub fn load(record: IdentityRecord) -> Result<Self> {
        let certificate = checked_certificate(record.certificate)?;
        ensure!(
            (1..=MAX_PRIVATE_KEY).contains(&record.private_key.len()),
            "invalid private key size"
        );
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(record.private_key));
        // Same stock parsing/matching routine as rustls's single-cert builders.
        let certified =
            CertifiedKey::from_der(vec![certificate.clone()], private_key, &provider())?;
        Ok(Self {
            certificate,
            certified,
        })
    }
    pub fn certificate(&self) -> &[u8] {
        self.certificate.as_ref()
    }
}

/// This adapter validates certificate syntax. It does not grant approval.
/// The owner must resolve persisted trust or explicit first-use approval before
/// constructing a TLS endpoint with this certificate.
pub struct PeerCertificate(CertificateDer<'static>);
impl PeerCertificate {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self(checked_certificate(bytes)?))
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        Sha256::digest(self.0.as_ref()).into()
    }
    pub fn peer_id(&self) -> blent_config::connection::pairing::PeerId {
        const DIGITS: &[u8] = b"0123456789abcdef";
        let mut text = String::with_capacity(64);
        for value in self.fingerprint() {
            text.push(DIGITS[(value >> 4) as usize] as char);
            text.push(DIGITS[(value & 15) as usize] as char);
        }
        blent_config::connection::pairing::PeerId::parse(text)
            .expect("SHA-256 formatting produces a valid peer ID")
    }
}
fn checked_certificate(bytes: Vec<u8>) -> Result<CertificateDer<'static>> {
    ensure!(
        (1..=MAX_CERTIFICATE).contains(&bytes.len()),
        "invalid certificate size"
    );
    let certificate = CertificateDer::from(bytes);
    RootCertStore::empty()
        .add(certificate.clone())
        .context("invalid peer certificate")?;
    Ok(certificate)
}
fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}
fn roots(peer: &PeerCertificate) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.add(peer.0.clone())?;
    Ok(roots)
}
fn server_config(identity: &Identity, peer: &PeerCertificate) -> Result<ServerConfig> {
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots(peer)?),
        provider(),
    )
    .build()?;
    let mut config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()?
        .with_client_cert_verifier(verifier)
        .with_cert_resolver(Arc::new(SingleCertAndKey::from(identity.certified.clone())));
    config.alpn_protocols = vec![ALPN.to_vec()];
    config.send_tls13_tickets = 0; // Every link must prove its current certificate.
    Ok(config)
}
fn client_config(identity: &Identity, peer: &PeerCertificate) -> Result<ClientConfig> {
    let mut config = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots(peer)?)
        .with_client_cert_resolver(Arc::new(SingleCertAndKey::from(identity.certified.clone())));
    config.alpn_protocols = vec![ALPN.to_vec()];
    config.resumption = rustls::client::Resumption::disabled();
    Ok(config)
}
fn verify_peer(
    certificates: Option<&[CertificateDer<'_>]>,
    alpn: Option<&[u8]>,
    peer: &PeerCertificate,
) -> Result<()> {
    ensure!(
        alpn.is_none_or(|protocol| protocol == ALPN),
        "unexpected connection protocol"
    );
    let certificate = certificates
        .and_then(|chain| chain.first())
        .context("missing authenticated peer")?;
    ensure!(
        Sha256::digest(certificate.as_ref()).as_slice() == peer.fingerprint(),
        "authenticated peer changed"
    );
    Ok(())
}
/// Exchange the fixed protocol/version only after certificate authentication.
/// API 27/28 peers lack public ALPN APIs; absence is safe only with this exchange.
async fn protocol<S: AsyncRead + AsyncWrite + Unpin>(stream: &mut S) -> Result<()> {
    stream.write_all(PREFACE).await?;
    stream.flush().await?;
    let mut received = [0; 10];
    stream.read_exact(&mut received).await?;
    ensure!(
        &received == PREFACE,
        "unexpected authenticated protocol preface"
    );
    Ok(())
}
/// The single deadline covers TLS, exact peer proof and protocol negotiation.
/// Application data is exposed only after all three have succeeded.
pub async fn accept<S: AsyncRead + AsyncWrite + Unpin>(
    io: S,
    identity: &Identity,
    peer: &PeerCertificate,
) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config(identity, peer)?));
    tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        let mut stream = acceptor.accept(io).await?;
        let (_, connection) = stream.get_ref();
        verify_peer(
            connection.peer_certificates(),
            connection.alpn_protocol(),
            peer,
        )?;
        protocol(&mut stream).await?;
        Ok(stream)
    })
    .await?
}
pub async fn connect<S: AsyncRead + AsyncWrite + Unpin>(
    io: S,
    identity: &Identity,
    peer: &PeerCertificate,
) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config(identity, peer)?));
    let name = ServerName::try_from("blent.local")?;
    tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        let mut stream = connector.connect(name, io).await?;
        let (_, connection) = stream.get_ref();
        verify_peer(
            connection.peer_certificates(),
            connection.alpn_protocol(),
            peer,
        )?;
        protocol(&mut stream).await?;
        Ok(stream)
    })
    .await?
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux"))]
mod persistence_tests;
