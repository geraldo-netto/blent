use super::super::super::transport;
use super::*;
use crate::secure_connection::{self, Identity, IdentityRecord, PeerCertificate};
fn identity() -> Identity {
    Identity::load(IdentityRecord::generate().unwrap()).unwrap()
}
fn peer(identity: &Identity) -> PeerCertificate {
    PeerCertificate::parse(identity.certificate().to_vec()).unwrap()
}
fn offer<S: AsyncRead + AsyncWrite + Unpin>(wire: &mut Transport<S>, message: Message) {
    let mut bytes = [0; 284];
    let count = message.encode(&mut bytes).unwrap();
    wire.multiplexer()
        .offer(Channel::Management, bytes[..count].to_vec())
        .unwrap();
}
fn command<S: AsyncRead + AsyncWrite + Unpin>(wire: &mut Transport<S>, message: Message) {
    match message {
        Message::Offer(key, _, _) => offer(wire, Message::Prepared(key)),
        Message::Commit(key) => {
            for channel in channels::members(key.resource) {
                wire.multiplexer()
                    .activate(*channel, key.generation)
                    .unwrap();
            }
            offer(wire, Message::Result(key, Outcome::Admitted));
            wire.multiplexer()
                .offer(Channel::Control, b"tablet control".to_vec())
                .unwrap();
        }
        other => panic!(
            "unexpected TLS peer command {}",
            matches!(other, Message::Revoke(..))
        ),
    }
}
pub(crate) async fn tablet<S: AsyncRead + AsyncWrite + Unpin>(stream: S) -> S {
    let mut wire = Transport::new(
        stream,
        Multiplexer::new(1, 1, 4096, 64, 64).unwrap(),
        Duration::from_secs(2),
    )
    .unwrap();
    let mut decoder = Decoder::new();
    let mut sent = false;
    let mut received = false;
    while !sent || !received {
        match wire.next().await.unwrap() {
            transport::Event::Written(header) => {
                sent |= header.channel == Channel::Control && header.kind == Kind::Data;
            }
            transport::Event::Received { header, deliver } => {
                if deliver {
                    received |= payload(&mut wire, &mut decoder, header);
                }
                wire.consume().unwrap();
            }
        }
    }
    wire.into_stream()
}
fn payload<S: AsyncRead + AsyncWrite + Unpin>(
    wire: &mut Transport<S>,
    decoder: &mut Decoder,
    header: Header,
) -> bool {
    let received = header.channel == Channel::Control;
    if received {
        assert_eq!(wire.payload(), b"host control");
    } else {
        let mut offset = 0;
        while offset < header.length as usize {
            let (count, message) = decoder.feed(&wire.payload()[offset..]).unwrap();
            offset += count;
            if let Some(message) = message {
                command(wire, message);
            }
        }
    }
    wire.multiplexer()
        .release(header.channel, header.length)
        .unwrap();
    received
}
#[tokio::test]
async fn t872_stock_mutual_tls_carries_admitted_duplex_control() {
    let host = identity();
    let tablet_id = identity();
    let host_pin = peer(&host);
    let tablet_pin = peer(&tablet_id);
    let (left, right) = tokio::io::duplex(256);
    let (server, client) = tokio::join!(
        secure_connection::accept(left, &host, &tablet_pin),
        secure_connection::connect(right, &tablet_id, &host_pin)
    );
    let mut session = Session::new(
        server.unwrap(),
        ticket(),
        Limits {
            channel_bytes: 4096,
            ..Limits::default()
        },
    )
    .unwrap();
    let lease = session
        .register(
            Resource::DisplaySession,
            1,
            Credential::new([2; 32]),
            Profile::Display,
            0,
            1000,
        )
        .unwrap();
    let (done, mut completion) = tokio::sync::oneshot::channel();
    let exchange = async {
        let mut admitted = false;
        let mut received = false;
        loop {
            tokio::select! {
                result = &mut completion, if received => { result.unwrap(); break; },
                event = session.next(1) => {
                    match event.unwrap() {
                        Event::State(_,State::Admitted) => { admitted = true; session.offer(&lease,Channel::Control,b"host control".to_vec()).unwrap(); },
                        Event::Readable(_,Channel::Control) => {
                            let mut bytes = [0;32]; let read = session.read(&lease,Channel::Control,&mut bytes).unwrap().unwrap();
                            assert!(admitted); assert_eq!(&bytes[..read.count],b"tablet control");
                            session.consumed(&lease,Channel::Control,read).unwrap(); received = true;
                        },
                        _ => {},
                    }
                }
            }
        }
        session
    };
    let peer_run = async {
        let stream = tablet(client.unwrap()).await;
        done.send(()).unwrap();
        stream
    };
    let (mut session, _peer) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(exchange, peer_run)
    })
    .await
    .unwrap();
    session.disconnect().unwrap();
    session.native_retired(&lease).unwrap();
}
