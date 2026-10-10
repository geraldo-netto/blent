use super::*;
use crate::broker::linux::{async_io::AsyncListener, owner::Owner, Socket};
use blent_config::connection::{control::Command, registration::Rejection};
use std::{future::Future, os::unix::fs::PermissionsExt};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .unwrap()
}
async fn pair() -> (AsyncSocket, AsyncSocket) {
    let root = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let listener = AsyncListener::new(Owner::bind(root.path(), 4).unwrap()).unwrap();
    let client = AsyncSocket::new(
        Socket::connect(&root.path().join("broker.sock"), unsafe { libc::geteuid() }).unwrap(),
    )
    .unwrap();
    let (server, _) = bounded(listener.accept()).await.unwrap();
    (client, server)
}
fn message(request: u64) -> Message {
    Message::Rejected {
        request,
        reason: Rejection::Busy,
    }
}
async fn retired(completion: Completion) {
    bounded(async {
        while !completion.is_complete() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}
#[tokio::test]
async fn t853_local_client_duplex_batches_and_actual_retirement() {
    let (client, socket) = pair().await;
    let mut peer = Peer::start(socket, Duration::from_secs(1)).unwrap();
    client
        .send(
            &Message::Control {
                request: 7,
                command: Command::Inspect,
            },
            &[],
        )
        .await
        .unwrap();
    assert_eq!(bounded(peer.next()).await.unwrap().message.request(), 7);
    peer.reply(vec![message(8), message(9)]).unwrap();
    assert_eq!(
        bounded(client.receive()).await.unwrap().message.request(),
        8
    );
    assert_eq!(
        bounded(client.receive()).await.unwrap().message.request(),
        9
    );
    let completion = peer.completion();
    client.close();
    assert!(bounded(peer.next()).await.is_err());
    assert!(peer.next().await.is_err());
    retired(completion).await;
}
#[tokio::test]
async fn t853_reply_capacity_and_timeout_bounds_reject_invalid_values() {
    for timeout in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        let (_, socket) = pair().await;
        assert!(Peer::start(socket, timeout).is_err());
    }
    let (_client, socket) = pair().await;
    let peer = Peer::start(socket, Duration::from_secs(1)).unwrap();
    assert!(peer.reply(vec![]).is_err());
    assert!(peer.reply(vec![message(1); MAX_BATCH + 1]).is_err());
    peer.reply(vec![message(1)]).unwrap();
    peer.reply(vec![message(2)]).unwrap();
    assert!(peer.reply(vec![message(3)]).is_err());
    let completion = peer.completion();
    peer.stop();
    drop(peer);
    retired(completion).await;
}
#[tokio::test]
async fn t853_stalled_client_writer_does_not_block_sibling() {
    let (_stalled, socket) = pair().await;
    let mut blocked = Peer::start(socket, Duration::from_millis(20)).unwrap();
    blocked.reply(vec![message(1); MAX_BATCH]).unwrap();
    let (client, socket) = pair().await;
    let mut sibling = Peer::start(socket, Duration::from_secs(1)).unwrap();
    sibling.reply(vec![message(2)]).unwrap();
    assert_eq!(
        bounded(client.receive()).await.unwrap().message.request(),
        2
    );
    assert!(bounded(blocked.next()).await.is_err());
    client.close();
    assert!(bounded(sibling.next()).await.is_err());
}
#[tokio::test]
async fn t853_closed_dispatch_lanes_retire_native_socket() {
    let (client, socket) = pair().await;
    let (output, input) = mpsc::channel(1);
    drop(input);
    client.send(&message(1), &[]).await.unwrap();
    assert!(bounded(read(&socket, output)).await.is_err());
    let (output, input) = mpsc::channel(1);
    drop(output);
    bounded(write(&socket, input, Duration::from_secs(1)))
        .await
        .unwrap();
    client.close();
    assert!(bounded(send_batch(&socket, vec![message(1)]))
        .await
        .is_err());
}
