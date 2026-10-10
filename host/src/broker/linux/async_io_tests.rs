use super::*;
use blent_config::connection::{
    management::{Credential, Profile, Resource},
    pairing::PeerId,
    registration::Rejection,
};
use std::net::TcpListener;
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .unwrap()
}

fn root() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap()
}
fn reply(request: u64) -> Message {
    Message::Rejected {
        request,
        reason: Rejection::Busy,
    }
}
async fn pair(listener: &AsyncListener, path: &std::path::Path) -> (AsyncSocket, AsyncSocket) {
    let client = Socket::connect(&path.join("broker.sock"), unsafe { libc::geteuid() }).unwrap();
    let (server, caller) = tokio::time::timeout(Duration::from_secs(2), listener.accept())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(caller.uid, unsafe { libc::geteuid() });
    (AsyncSocket::new(client).unwrap(), server)
}

#[tokio::test]
async fn t874_async_duplex_clients_progress_without_global_io_lock() {
    bounded(async {
        let root = root();
        let listener = AsyncListener::new(Owner::bind(root.path(), 4).unwrap()).unwrap();
        let (first, server) = pair(&listener, root.path()).await;
        let (second, sibling) = pair(&listener, root.path()).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(5), first.receive())
                .await
                .is_err()
        );
        let exchange = async {
            let to_server = reply(1);
            let to_client = reply(2);
            let (sent, received, sent_back, received_back) = tokio::join!(
                second.send(&to_server, &[]),
                sibling.receive(),
                sibling.send(&to_client, &[]),
                second.receive()
            );
            sent.unwrap();
            sent_back.unwrap();
            assert_eq!(received.unwrap().message.request(), 1);
            assert_eq!(received_back.unwrap().message.request(), 2);
        };
        tokio::time::timeout(Duration::from_secs(2), exchange)
            .await
            .unwrap();
        first.close();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), server.receive())
                .await
                .unwrap()
                .is_err()
        );
        second.send(&reply(3), &[]).await.unwrap();
        assert_eq!(sibling.receive().await.unwrap().message.request(), 3);
    })
    .await;
}

#[tokio::test]
async fn t874_async_descriptor_ownership_outlives_client_and_listener() {
    bounded(async {
        let root = root();
        let listener = AsyncListener::new(Owner::bind(root.path(), 2).unwrap()).unwrap();
        let (client, server) = pair(&listener, root.path()).await;
        let video = TcpListener::bind("127.0.0.1:0").unwrap();
        let control = TcpListener::bind("127.0.0.1:0").unwrap();
        let addresses = [video.local_addr().unwrap(), control.local_addr().unwrap()];
        let message = Message::Register {
            request: 9,
            peer: PeerId::parse("ab".repeat(32)).unwrap(),
            resource: Resource::DisplaySession,
            credential: Credential::new([1; 32]),
            profile: Profile::Display,
        };
        client
            .send(&message, &[video.as_fd(), control.as_fd()])
            .await
            .unwrap();
        let packet = server.receive().await.unwrap();
        assert_eq!(packet.descriptors.len(), 2);
        drop(video);
        drop(control);
        client.close();
        drop(listener);
        assert!(TcpListener::bind(addresses[0]).is_err());
        assert!(TcpListener::bind(addresses[1]).is_err());
        drop(packet);
        assert!(TcpListener::bind(addresses[0]).is_ok());
        assert!(TcpListener::bind(addresses[1]).is_ok());
    })
    .await;
}

#[tokio::test]
async fn t874_invalid_send_closes_socket_without_consuming_caller_descriptors() {
    bounded(async {
        for malformed in [false, true] {
            let root = root();
            let listener = AsyncListener::new(Owner::bind(root.path(), 1).unwrap()).unwrap();
            let (client, server) = pair(&listener, root.path()).await;
            let file = tempfile::tempfile().unwrap();
            let result = if malformed {
                client.send(&reply(0), &[]).await
            } else {
                client.send(&reply(1), &[file.as_fd()]).await
            };
            assert!(result.is_err());
            assert!(file.metadata().is_ok());
            assert!(client.send(&reply(1), &[]).await.is_err());
            assert!(server.receive().await.is_err());
            client.close();
        }
    })
    .await;
}

#[tokio::test]
async fn t874_malformed_received_packet_retires_transferred_descriptors() {
    bounded(async {
        for bytes in [b"bad".to_vec(), reply(1).encode().unwrap()] {
            let root = root();
            let listener = AsyncListener::new(Owner::bind(root.path(), 1).unwrap()).unwrap();
            let (client, server) = pair(&listener, root.path()).await;
            let endpoint = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = endpoint.local_addr().unwrap();
            client
                .socket
                .get_ref()
                .send(&bytes, &[endpoint.as_raw_fd()])
                .unwrap();
            drop(endpoint);
            assert!(server.receive().await.is_err());
            assert!(server.receive().await.is_err());
            assert!(TcpListener::bind(address).is_ok());
        }
    })
    .await;
}

#[tokio::test]
async fn t874_close_wakes_pending_receive_and_accept_cancellation_keeps_election() {
    bounded(async {
        let root = root();
        let listener = AsyncListener::new(Owner::bind(root.path(), 1).unwrap()).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(5), listener.accept())
                .await
                .is_err()
        );
        assert!(Owner::bind(root.path(), 1).is_err());
        let (client, server) = pair(&listener, root.path()).await;
        let receive = client.receive();
        tokio::pin!(receive);
        assert!(tokio::time::timeout(Duration::from_millis(5), &mut receive)
            .await
            .is_err());
        client.close();
        assert!(unsafe { libc::fcntl(client.socket.get_ref().as_raw_fd(), libc::F_GETFD) } >= 0);
        assert!(tokio::time::timeout(Duration::from_secs(2), receive)
            .await
            .unwrap()
            .is_err());
        assert!(server.receive().await.is_err());
    })
    .await;
}

#[tokio::test]
async fn t874_spurious_readiness_clears_then_new_packet_wakes_receiver() {
    bounded(async {
        let root = root();
        let listener = AsyncListener::new(Owner::bind(root.path(), 1).unwrap()).unwrap();
        let (client, server) = pair(&listener, root.path()).await;
        server.send(&reply(1), &[]).await.unwrap();
        let readiness = client.socket.readable().await.unwrap();
        assert!(readiness.ready().is_readable());
        let removed = client.socket.get_ref().receive_message().unwrap().unwrap();
        assert_eq!(removed.message.request(), 1);
        // A competing native consumer exhausted the FD without clearing the
        // cached reactor readiness. The adapter must re-arm instead of spin.
        drop(readiness);
        let receive = client.receive();
        tokio::pin!(receive);
        assert!(tokio::time::timeout(Duration::from_millis(5), &mut receive)
            .await
            .is_err());
        server.send(&reply(2), &[]).await.unwrap();
        assert_eq!(receive.await.unwrap().message.request(), 2);
    })
    .await;
}

fn saturate(socket: &AsyncSocket) -> usize {
    let bytes = reply(1).encode().unwrap();
    for count in 0..10000 {
        match socket.socket.get_ref().send(&bytes, &[]) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return count,
            Err(error) => panic!("unexpected packet failure: {error}"),
        }
    }
    panic!("bounded socket saturation fixture exceeded limit");
}

#[tokio::test]
async fn t874_backpressure_preserves_bidirectional_progress_and_cancelled_send() {
    bounded(async {
        let root = root();
        let listener = AsyncListener::new(Owner::bind(root.path(), 1).unwrap()).unwrap();
        let (client, server) = pair(&listener, root.path()).await;
        let queued = saturate(&client);
        assert!(queued > 0);
        let message = reply(2);
        let send = client.send(&message, &[]);
        tokio::pin!(send);
        assert!(tokio::time::timeout(Duration::from_millis(5), &mut send)
            .await
            .is_err());
        server.send(&reply(3), &[]).await.unwrap();
        assert_eq!(client.receive().await.unwrap().message.request(), 3);
        for _ in 0..queued {
            assert_eq!(server.receive().await.unwrap().message.request(), 1);
        }
        tokio::time::timeout(Duration::from_secs(2), send)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.receive().await.unwrap().message.request(), 2);
        saturate(&client);
        let blocked = client.send(&message, &[]);
        tokio::pin!(blocked);
        assert!(tokio::time::timeout(Duration::from_millis(5), &mut blocked)
            .await
            .is_err());
        client.close();
        assert!(tokio::time::timeout(Duration::from_secs(2), blocked)
            .await
            .unwrap()
            .is_err());
    })
    .await;
}
