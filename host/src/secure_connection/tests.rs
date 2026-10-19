use super::*;
use std::time::Duration;
// Independent oracle for the unchanged public default.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
use tokio::io::{AsyncReadExt, AsyncWriteExt};
fn identity() -> Identity {
    Identity::load(IdentityRecord::generate().unwrap()).unwrap()
}
fn peer(identity: &Identity) -> PeerCertificate {
    PeerCertificate::parse(identity.certificate().to_vec()).unwrap()
}

#[tokio::test]
async fn t846_mutually_pinned_tls_carries_duplex_bytes() {
    let host = identity();
    let tablet = identity();
    let host_peer = peer(&host);
    let tablet_peer = peer(&tablet);
    let (left, right) = tokio::io::duplex(1024);
    let (server, client) = tokio::join!(
        accept(
            left,
            &host,
            &tablet_peer,
            blent_config::connection::handshake::Timeout::default()
        ),
        connect(
            right,
            &tablet,
            &host_peer,
            blent_config::connection::handshake::Timeout::default()
        )
    );
    let mut server = server.unwrap();
    let mut client = client.unwrap();
    let sending = async {
        server.write_all(b"host bytes").await.unwrap();
        server.flush().await.unwrap();
        let mut reply = [0; 12];
        server.read_exact(&mut reply).await.unwrap();
        assert_eq!(&reply, b"tablet bytes");
    };
    let receiving = async {
        let mut value = [0; 10];
        client.read_exact(&mut value).await.unwrap();
        assert_eq!(&value, b"host bytes");
        client.write_all(b"tablet bytes").await.unwrap();
        client.flush().await.unwrap();
    };
    tokio::join!(sending, receiving);
}
#[tokio::test]
async fn t846_unapproved_peer_cannot_complete_tls() {
    let host = identity();
    let tablet = identity();
    let stranger = identity();
    let host_peer = peer(&host);
    let wrong_peer = peer(&stranger);
    let (left, right) = tokio::io::duplex(8192);
    let (server, _client) = tokio::join!(
        accept(
            left,
            &host,
            &wrong_peer,
            blent_config::connection::handshake::Timeout::default()
        ),
        connect(
            right,
            &tablet,
            &host_peer,
            blent_config::connection::handshake::Timeout::default()
        )
    );
    assert!(server.is_err());
    let (left, right) = tokio::io::duplex(8192);
    let tablet_peer = peer(&tablet);
    let (_server, client) = tokio::join!(
        accept(
            left,
            &host,
            &tablet_peer,
            blent_config::connection::handshake::Timeout::default()
        ),
        connect(
            right,
            &tablet,
            &wrong_peer,
            blent_config::connection::handshake::Timeout::default()
        )
    );
    assert!(client.is_err());
}
#[tokio::test(start_paused = true)]
async fn t846_silent_peer_handshake_has_deadline() {
    let host = identity();
    let tablet = identity();
    let peer = peer(&tablet);
    let (left, _silent) = tokio::io::duplex(1024);
    assert!(accept(
        left,
        &host,
        &peer,
        blent_config::connection::handshake::Timeout::default()
    )
    .await
    .is_err());
    let (left, _silent) = tokio::io::duplex(1024);
    assert!(connect(
        left,
        &host,
        &peer,
        blent_config::connection::handshake::Timeout::default()
    )
    .await
    .is_err());
}
#[test]
fn t846_identity_and_certificate_bounds_and_pin_checks() {
    let identity = identity();
    let peer = peer(&identity);
    let certificate = CertificateDer::from(identity.certificate());
    verify_peer(Some(&[certificate.clone()]), Some(ALPN), &peer).unwrap();
    assert!(verify_peer(None, Some(ALPN), &peer).is_err());
    assert!(verify_peer(Some(&[]), Some(ALPN), &peer).is_err());
    verify_peer(Some(&[certificate.clone()]), None, &peer).unwrap();
    assert!(verify_peer(Some(&[certificate]), Some(b"other/1"), &peer).is_err());
    assert!(verify_peer(Some(&[CertificateDer::from(vec![0; 8])]), Some(ALPN), &peer).is_err());
    for size in [0, 1, 4096, 4097, 8192] {
        assert!(PeerCertificate::parse(vec![0; size]).is_err());
    }
    for size in [0, 1, 4096, 4097] {
        assert!(Identity::load(IdentityRecord {
            certificate: identity.certificate().to_vec(),
            private_key: vec![0; size]
        })
        .is_err());
    }
    let mut record = IdentityRecord::generate().unwrap();
    record.certificate = identity.certificate().to_vec();
    assert!(Identity::load(record).is_err());
}

#[tokio::test]
async fn t846_no_alpn_requires_authenticated_protocol_preface() {
    let host = identity();
    let tablet = identity();
    let host_peer = peer(&host);
    let tablet_peer = peer(&tablet);
    let (left, right) = tokio::io::duplex(8192);
    let mut config = client_config(&tablet, &host_peer).unwrap();
    config.alpn_protocols.clear();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let client = async {
        let mut stream = connector
            .connect(ServerName::try_from("blent.local").unwrap(), right)
            .await
            .unwrap();
        let expected = include_bytes!("../../../testdata/connection-tls/protocol-v1.bin");
        stream.write_all(expected).await.unwrap();
        stream.flush().await.unwrap();
        let mut received = [0; 10];
        stream.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, expected);
        stream.write_all(b"after preface").await.unwrap();
        stream.flush().await.unwrap();
    };
    let server = async {
        let mut stream = accept(
            left,
            &host,
            &tablet_peer,
            blent_config::connection::handshake::Timeout::default(),
        )
        .await
        .unwrap();
        let mut value = [0; 13];
        stream.read_exact(&mut value).await.unwrap();
        assert_eq!(&value, b"after preface");
    };
    tokio::join!(server, client);
}
#[tokio::test]
async fn t846_matching_alpn_does_not_bypass_protocol_preface() {
    for wrong in [b"wrongproto".as_slice(), b"BLENTUSB\0\x02", b""] {
        let host = identity();
        let tablet = identity();
        let host_peer = peer(&host);
        let tablet_peer = peer(&tablet);
        let (left, right) = tokio::io::duplex(8192);
        let connector =
            tokio_rustls::TlsConnector::from(Arc::new(client_config(&tablet, &host_peer).unwrap()));
        let client = async {
            let mut stream = connector
                .connect(ServerName::try_from("blent.local").unwrap(), right)
                .await
                .unwrap();
            let _ = stream.write_all(wrong).await;
            let _ = stream.flush().await;
        };
        let (server, ()) = tokio::join!(
            accept(
                left,
                &host,
                &tablet_peer,
                blent_config::connection::handshake::Timeout::default()
            ),
            client
        );
        assert!(
            server.is_err(),
            "T846: TLS alone admitted missing/wrong application protocol"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn t846_preface_wait_shares_handshake_deadline_in_both_roles() {
    let host = identity();
    let tablet = identity();
    let host_peer = peer(&host);
    let tablet_peer = peer(&tablet);
    let (left, right) = tokio::io::duplex(8192);
    let connector =
        tokio_rustls::TlsConnector::from(Arc::new(client_config(&tablet, &host_peer).unwrap()));
    let silent = async {
        let mut stream = connector
            .connect(ServerName::try_from("blent.local").unwrap(), right)
            .await
            .unwrap();
        let mut preface = [0; 10];
        stream.read_exact(&mut preface).await.unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
    };
    let checked = async {
        let start = tokio::time::Instant::now();
        assert!(accept(
            left,
            &host,
            &tablet_peer,
            blent_config::connection::handshake::Timeout::default()
        )
        .await
        .is_err());
        assert_eq!(start.elapsed(), HANDSHAKE_TIMEOUT);
    };
    tokio::join!(checked, silent);
    let (left, right) = tokio::io::duplex(8192);
    let acceptor =
        tokio_rustls::TlsAcceptor::from(Arc::new(server_config(&host, &tablet_peer).unwrap()));
    let silent = async {
        let mut stream = acceptor.accept(left).await.unwrap();
        let mut preface = [0; 10];
        stream.read_exact(&mut preface).await.unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
    };
    let checked = async {
        let start = tokio::time::Instant::now();
        assert!(connect(
            right,
            &tablet,
            &host_peer,
            blent_config::connection::handshake::Timeout::default()
        )
        .await
        .is_err());
        assert_eq!(start.elapsed(), HANDSHAKE_TIMEOUT);
    };
    tokio::join!(silent, checked);
}
#[tokio::test]
async fn t846_preface_golden_vector_and_bounded_mutations() {
    assert_eq!(
        PREFACE,
        include_bytes!("../../../testdata/connection-tls/protocol-v1.bin")
    );
    for offset in 0..PREFACE.len() {
        let (mut owned, mut peer) = tokio::io::duplex(128);
        let mut wrong = *PREFACE;
        wrong[offset] ^= 0xff;
        peer.write_all(&wrong).await.unwrap();
        assert!(protocol(&mut owned).await.is_err());
    }
    let (mut owned, peer) = tokio::io::duplex(128);
    drop(peer);
    assert!(protocol(&mut owned).await.is_err());
}
