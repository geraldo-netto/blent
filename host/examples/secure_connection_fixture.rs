//! T846 test-only loopback peer. Uses committed PUBLIC fixture keys, never users' identities.
use anyhow::{ensure, Context, Result};
use blent::secure_connection::{accept, Identity, IdentityRecord, PeerCertificate};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::{io::Write, path::Path, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

fn record(directory: &Path) -> Result<IdentityRecord> {
    Ok(IdentityRecord {
        certificate: std::fs::read(directory.join("host.der"))?,
        private_key: std::fs::read(directory.join("host.pk8"))?,
    })
}

// Deliberately wrong protocol after valid mutual TLS, testing Android's gate.
async fn wrong_preface(socket: TcpStream, directory: &Path) -> Result<()> {
    let identity = record(directory)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from(std::fs::read(
        directory.join("tablet.der"),
    )?))?;
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()?;
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![CertificateDer::from(identity.certificate)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.private_key)),
        )?;
    config.alpn_protocols = vec![b"blent-usb/1".to_vec()];
    config.send_tls13_tickets = 0;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let mut stream = acceptor.accept(socket).await?;
    stream.write_all(b"wrongproto").await?;
    stream.flush().await?;
    Ok(())
}

async fn serve(directory: &Path, ready: &Path, mode: &str) -> Result<()> {
    let identity = Identity::load(record(directory)?)?;
    let peer = PeerCertificate::parse(std::fs::read(directory.join("tablet.der"))?)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(ready)?;
    writeln!(file, "{}", listener.local_addr()?.port())?;
    drop(file);
    let (socket, address) = listener.accept().await?;
    ensure!(
        address.ip().is_loopback(),
        "fixture accepts only loopback peers"
    );
    if mode == "wrong-preface" {
        return wrong_preface(socket, directory).await;
    }
    if mode == "setup" {
        return setup_session(socket, &identity, &peer).await;
    }
    let mut stream = accept(socket, &identity, &peer).await?;
    if matches!(mode, "window" | "session") {
        let limits = blent::broker::session::Limits {
            channel_bytes: 65536,
            io_timeout: Duration::from_secs(3),
            ..Default::default()
        };
        let selected = blent::broker::window::negotiate(&mut stream, limits).await?;
        ensure!(
            selected.channel_bytes == 4096,
            "tablet window selection mismatch"
        );
        if mode == "session" {
            return resource_session(stream, &peer, selected).await;
        }
        stream = framed(stream).await?;
    }
    let mut request = [0; 7];
    stream.read_exact(&mut request).await?;
    ensure!(
        &request == b"fixture",
        "invalid fixture application request"
    );
    stream.write_all(b"accepted").await?;
    stream.flush().await?;
    Ok(())
}

// T886: this owner has only the fixture's in-process TcpStream, which is dropped
// before retirement. It never claims USB or subprocess retirement evidence.
struct SetupOwner(blent_config::retirement::Admission);
impl blent::accessory::Owner for SetupOwner {
    fn completion(&self) -> blent_config::retirement::Completion {
        self.0.completion()
    }
    fn retire(
        self: Box<Self>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}

// Use the maintained host setup/authority path, rather than skipping directly
// to TLS. The fixture approves only its committed public tablet certificate.
async fn setup_session(
    socket: TcpStream,
    identity: &Identity,
    peer: &PeerCertificate,
) -> Result<()> {
    use blent::accessory::{setup, Resource};
    use blent_config::{connection::pairing::Pairing, retirement::Pool};
    let owner = Box::new(SetupOwner(Pool::new(1)?.reserve()?));
    let resource = Resource {
        stream: Box::new(socket),
        owner,
    };
    let candidate = setup::discover(resource, identity, Duration::from_secs(3)).await?;
    let proven = peer.peer_id();
    ensure!(
        candidate.peer() == &proven,
        "unexpected setup fixture identity"
    );
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(proven.clone(), true, 0, 10000)?;
    let verified = candidate
        .authenticate_with_timeout(identity, ticket.clone(), Duration::from_secs(3))
        .await?;
    let approved = pairing.authenticated(&ticket, &verified.peer, 0)?;
    ensure!(
        pairing.current(&approved),
        "setup fixture admission expired"
    );
    let Resource { stream, owner } = verified.resource;
    let result = setup_resources(stream, peer).await;
    owner.retire().await?;
    result
}

async fn setup_resources(
    mut stream: Box<dyn blent::accessory::Stream>,
    peer: &PeerCertificate,
) -> Result<()> {
    let limits = blent::broker::session::Limits {
        channel_bytes: 65536,
        io_timeout: Duration::from_secs(3),
        ..Default::default()
    };
    let selected = blent::broker::window::negotiate(&mut stream, limits).await?;
    ensure!(
        selected.channel_bytes == 4096,
        "setup window selection mismatch"
    );
    resource_session(stream, peer, selected).await
}

// One bounded framing phase, followed by the existing raw fixture exchange.
// Exercises the real transport/mux; no media Session or resource authority.
async fn framed<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(stream: S) -> Result<S> {
    use blent::broker::transport::{Event, Transport};
    use blent_config::connection::{
        mux::Multiplexer,
        wire::{Channel, Kind},
    };
    let mut transport = Transport::new(
        stream,
        Multiplexer::new(1, 1, 4096, 8, 64)?,
        Duration::from_secs(3),
    )?;
    let mut received = false;
    for _ in 0..16 {
        match transport.next().await? {
            Event::Written(header) if header.kind == Kind::Data => {
                ensure!(
                    received && header.channel == Channel::Management,
                    "unexpected framed reply"
                );
                return Ok(transport.into_stream());
            }
            Event::Written(_) => (),
            Event::Received { header, deliver } => {
                ensure!(
                    header.channel == Channel::Management,
                    "unexpected fixture channel"
                );
                if deliver {
                    ensure!(
                        !received && transport.payload() == b"window-frame",
                        "invalid framed request"
                    );
                    received = true;
                    transport
                        .multiplexer()
                        .release(header.channel, header.length)?;
                    ensure!(
                        transport
                            .multiplexer()
                            .offer(Channel::Management, b"framed-ok".to_vec())
                            .is_ok(),
                        "fixture reply queue full"
                    );
                }
                transport.consume()?;
            }
        }
    }
    anyhow::bail!("fixture framing turn limit exceeded")
}

// T882 real host resource policy against the maintained Kotlin tablet session.
async fn resource_session<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: S,
    peer: &PeerCertificate,
    limits: blent::broker::session::Limits,
) -> Result<()> {
    use blent_config::connection::pairing::{Pairing, PeerId};
    let fingerprint = peer
        .fingerprint()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let proven = PeerId::parse(fingerprint)?;
    let mut pairing = Pairing::default();
    let ticket = pairing.begin(proven.clone(), true, 0, 3000)?;
    let authority = pairing.authenticated(&ticket, &proven, 0)?;
    ensure!(pairing.current(&authority), "fixture TLS authority missing");
    let mut session = blent::broker::session::Session::new(stream, ticket, limits)?;
    for generation in 1..=2 {
        resource_generation(&mut session, generation).await?;
    }
    ensure!(
        session.capabilities() == Some(15),
        "tablet capabilities mismatch"
    );
    Ok(())
}

async fn resource_generation<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    session: &mut blent::broker::session::Session<S>,
    generation: u64,
) -> Result<()> {
    use blent::broker::session::{Event, State};
    use blent_config::connection::management::{Credential, Profile, Reason, Resource};
    let lease = session.register(
        Resource::DisplaySession,
        generation,
        Credential::new([7; 32]),
        Profile::Display,
        0,
        3000,
    )?;
    ensure!(
        lease.key.generation == generation,
        "replacement generation mismatch"
    );
    let mut received = 0;
    for _ in 0..128 {
        match session.next(0).await? {
            Event::State(current, State::Admitted) => {
                ensure!(current == lease, "wrong admitted lease");
                session_payloads(session, &lease)?;
            }
            Event::Readable(key, channel) => {
                ensure!(key == lease.key, "wrong readable generation");
                received |= session_echo(session, &lease, channel)?;
                if received == 3 {
                    session.cancel(&lease, Reason::Requested)?;
                    session.native_retired(&lease)?;
                }
            }
            Event::State(current, State::Retired) => {
                ensure!(
                    current == lease && received == 3,
                    "incomplete resource retirement"
                );
                return Ok(());
            }
            _ => (),
        }
    }
    anyhow::bail!("fixture resource turn limit exceeded")
}

fn session_payloads<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    session: &mut blent::broker::session::Session<S>,
    lease: &blent_config::connection::resources::Lease,
) -> Result<()> {
    use blent_config::connection::wire::Channel;
    for channel in [Channel::Control, Channel::Display] {
        ensure!(
            session
                .offer(
                    lease,
                    channel,
                    vec![lease.key.generation as u8, channel as u8, 42]
                )
                .is_ok(),
            "fixture resource output refused"
        );
    }
    Ok(())
}

fn session_echo<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    session: &mut blent::broker::session::Session<S>,
    lease: &blent_config::connection::resources::Lease,
    channel: blent_config::connection::wire::Channel,
) -> Result<u8> {
    use blent_config::connection::wire::Channel;
    ensure!(
        matches!(channel, Channel::Control | Channel::Display),
        "unexpected fixture lane"
    );
    let mut bytes = [0; 3];
    let read = session
        .read(lease, channel, &mut bytes)?
        .context("missing fixture echo")?;
    ensure!(
        read.count == 3 && bytes == [lease.key.generation as u8, channel as u8, 42],
        "fixture echo changed"
    );
    session.consumed(lease, channel, read)?;
    Ok(1 << (channel as u8 - 1))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(args.len() == 4, "usage: secure_connection_fixture PUBLIC_FIXTURE_DIRECTORY NEW_READY_FILE accept|wrong-preface|window|session|setup");
    let mode = args[3].to_str().context("invalid mode")?;
    ensure!(
        matches!(
            mode,
            "accept" | "wrong-preface" | "window" | "session" | "setup"
        ),
        "invalid fixture mode"
    );
    tokio::time::timeout(
        Duration::from_secs(20),
        serve(Path::new(&args[1]), Path::new(&args[2]), mode),
    )
    .await?
}
