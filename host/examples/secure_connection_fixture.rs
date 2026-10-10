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
    let mut stream = accept(socket, &identity, &peer).await?;
    if mode == "window" {
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(args.len() == 4, "usage: secure_connection_fixture PUBLIC_FIXTURE_DIRECTORY NEW_READY_FILE accept|wrong-preface|window");
    let mode = args[3].to_str().context("invalid mode")?;
    ensure!(
        matches!(mode, "accept" | "wrong-preface" | "window"),
        "invalid fixture mode"
    );
    tokio::time::timeout(
        Duration::from_secs(20),
        serve(Path::new(&args[1]), Path::new(&args[2]), mode),
    )
    .await?
}
