//! T1104: independent requested timers must bound the original TLS operation.
use super::*;
use std::time::Duration;

fn identity() -> Identity {
    Identity::load(IdentityRecord::generate().unwrap()).unwrap()
}
fn peer(identity: &Identity) -> PeerCertificate {
    PeerCertificate::parse(identity.certificate().to_vec()).unwrap()
}
async fn selected_silence(server: bool, milliseconds: u64) {
    let local = identity();
    let remote = identity();
    let approved = peer(&remote);
    let (owned, _silent) = tokio::io::duplex(8192);
    let timeout = HandshakeTimeout::new(Duration::from_millis(milliseconds)).unwrap();
    let mut expected = Box::pin(tokio::time::sleep(Duration::from_millis(milliseconds)));
    assert!(futures_util::poll!(&mut expected).is_pending());
    let mut operation = Box::pin(open(server, owned, &local, &approved, timeout));
    assert!(futures_util::poll!(&mut operation).is_pending());
    tokio::time::advance(Duration::from_millis(milliseconds - 1)).await;
    assert!(futures_util::poll!(&mut operation).is_pending());
    tokio::time::advance(Duration::from_millis(1)).await;
    expected.await;
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut operation) else {
        panic!("T1104 selected TLS timeout ignored");
    };
    assert!(result
        .unwrap_err()
        .downcast_ref::<tokio::time::error::Elapsed>()
        .is_some());
}
#[tokio::test(start_paused = true)]
async fn t1104_selected_tls_deadline_bounds_both_roles_before_native_reuse() {
    for server in [true, false] {
        for milliseconds in [1, 37, 60_000] {
            selected_silence(server, milliseconds).await;
        }
    }
}

async fn open(
    server: bool,
    stream: tokio::io::DuplexStream,
    local: &Identity,
    approved: &PeerCertificate,
    timeout: HandshakeTimeout,
) -> Result<()> {
    if server {
        accept(stream, local, approved, timeout).await.map(|_| ())
    } else {
        connect(stream, local, approved, timeout).await.map(|_| ())
    }
}
async fn delayed_peer(server: bool) {
    let host = identity();
    let tablet = identity();
    let host_peer = peer(&host);
    let tablet_peer = peer(&tablet);
    let (left, right) = tokio::io::duplex(8192);
    let selected = HandshakeTimeout::new(Duration::from_secs(20)).unwrap();
    let server_budget = if server {
        selected
    } else {
        HandshakeTimeout::default()
    };
    let client_budget = if server {
        HandshakeTimeout::default()
    } else {
        selected
    };
    let accept = async {
        if !server {
            tokio::time::sleep(Duration::from_secs(11)).await;
        }
        open(true, left, &host, &tablet_peer, server_budget).await
    };
    let connect = async {
        if server {
            tokio::time::sleep(Duration::from_secs(11)).await;
        }
        open(false, right, &tablet, &host_peer, client_budget).await
    };
    let (accepted, connected) = tokio::join!(accept, connect);
    accepted.unwrap();
    connected.unwrap();
}
#[tokio::test(start_paused = true)]
async fn t1104_selected_longer_budget_accepts_delayed_peer_in_both_roles() {
    delayed_peer(true).await;
    delayed_peer(false).await;
}
async fn read_preface(
    server: bool,
    stream: tokio::io::DuplexStream,
    local: &Identity,
    approved: &PeerCertificate,
) -> Box<dyn crate::accessory::Stream> {
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mut stream: Box<dyn crate::accessory::Stream> = if server {
        let connector =
            tokio_rustls::TlsConnector::from(Arc::new(client_config(local, approved).unwrap()));
        Box::new(
            connector
                .connect(ServerName::try_from("blent.local").unwrap(), stream)
                .await
                .unwrap(),
        )
    } else {
        let acceptor =
            tokio_rustls::TlsAcceptor::from(Arc::new(server_config(local, approved).unwrap()));
        Box::new(acceptor.accept(stream).await.unwrap())
    };
    let mut received = [0; 10];
    stream.read_exact(&mut received).await.unwrap();
    assert_eq!(&received, PREFACE);
    stream
}
async fn incomplete_preface(server: bool) {
    let local = identity();
    let remote = identity();
    let approved = peer(&remote);
    let remote_approved = peer(&local);
    let (owned, other) = tokio::io::duplex(8192);
    let selected = HandshakeTimeout::new(Duration::from_millis(37)).unwrap();
    let mut expected = Box::pin(tokio::time::sleep(Duration::from_millis(37)));
    assert!(futures_util::poll!(&mut expected).is_pending());
    let mut operation = Box::pin(open(server, owned, &local, &approved, selected));
    let _silent = tokio::select! {
        result = &mut operation => panic!("T1104 operation ended before authenticated preface: {result:?}"),
        stream = read_preface(server, other, &remote, &remote_approved) => stream,
    };
    assert!(futures_util::poll!(&mut operation).is_pending());
    expected.await;
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut operation) else {
        panic!("T1104 authenticated preface received a fresh deadline");
    };
    assert!(result
        .unwrap_err()
        .downcast_ref::<tokio::time::error::Elapsed>()
        .is_some());
}
#[tokio::test(start_paused = true)]
async fn t1104_tls_and_uncompleted_preface_share_original_selected_deadline() {
    incomplete_preface(true).await;
    incomplete_preface(false).await;
}
