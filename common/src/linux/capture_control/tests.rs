use super::*;
use std::future::Future;
use std::time::Duration;

async fn pending(future: impl Future) {
    assert!(tokio::time::timeout(Duration::from_millis(10), future)
        .await
        .is_err());
}

async fn command(peer: &Inherited) -> Request {
    loop {
        let mut bytes = [0; RECORD_BYTES];
        if let Some(received) = packet::receive(peer.0.as_raw_fd(), &mut bytes).unwrap() {
            assert_eq!(received.length, RECORD_BYTES);
            assert!(received.descriptors.is_empty());
            return Request::decode(&bytes).unwrap();
        }
        tokio::task::yield_now().await;
    }
}

fn acknowledge(peer: &Inherited, request: Request) {
    packet::send(peer.0.as_raw_fd(), &request.encode(), &[]).unwrap();
}

#[tokio::test]
async fn t1023_cancellation_keeps_original_command_until_exact_echo() {
    let (mut control, peer) = Control::pair().unwrap();
    pending(control.set_active(true)).await;
    let first = command(&peer).await;
    assert_eq!(first, Request::after(0, true).unwrap());
    pending(control.set_active(false)).await;
    let mut bytes = [0; RECORD_BYTES];
    assert!(packet::receive(peer.0.as_raw_fd(), &mut bytes)
        .unwrap()
        .is_none());
    acknowledge(&peer, first);
    pending(control.set_active(false)).await;
    let second = command(&peer).await;
    assert_eq!(second, Request::after(1, false).unwrap());
    acknowledge(&peer, second);
    assert_eq!(control.set_active(false).await.unwrap(), second);
    assert_eq!(control.set_active(false).await.unwrap(), second);
    assert!(packet::receive(peer.0.as_raw_fd(), &mut bytes)
        .unwrap()
        .is_none());
}

async fn rejects(bytes: &[u8], descriptors: &[i32]) {
    let (mut control, peer) = Control::pair().unwrap();
    pending(control.set_active(true)).await;
    let correct = command(&peer).await;
    packet::send(peer.0.as_raw_fd(), bytes, descriptors).unwrap();
    assert!(control.set_active(true).await.is_err());
    acknowledge(&peer, correct);
    assert!(control.set_active(true).await.is_err());
}

#[tokio::test]
async fn t1023_malformed_foreign_truncated_and_descriptor_echo_poison() {
    for length in 0..=32 {
        rejects(&vec![0xff; length], &[]).await;
    }
    rejects(&Request::after(0, false).unwrap().encode(), &[]).await;
    rejects(&Request::after(1, true).unwrap().encode(), &[]).await;
    let descriptor = tempfile::tempfile().unwrap();
    let before = aliases(&descriptor);
    rejects(
        &Request::after(0, true).unwrap().encode(),
        &[descriptor.as_raw_fd()],
    )
    .await;
    assert_eq!(aliases(&descriptor), before);
}

fn aliases(file: &std::fs::File) -> usize {
    use std::os::unix::fs::MetadataExt;
    let expected = file.metadata().unwrap();
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| std::fs::metadata(entry.ok()?.path()).ok())
        .filter(|actual| (actual.dev(), actual.ino()) == (expected.dev(), expected.ino()))
        .count()
}

#[tokio::test]
async fn t1023_closed_peer_cannot_acknowledge_pending_transition() {
    let (mut control, peer) = Control::pair().unwrap();
    pending(control.set_active(true)).await;
    assert!(command(&peer).await.active());
    drop(peer);
    assert!(control.set_active(true).await.is_err());
    assert!(control.completed.is_none());
    assert!(control.set_active(false).await.is_err());
}

#[tokio::test]
async fn t1023_peer_close_and_sequence_exhaustion_are_terminal() {
    let (mut control, peer) = Control::pair().unwrap();
    drop(peer);
    assert!(control.set_active(true).await.is_err());
    assert!(control.set_active(false).await.is_err());
    let (mut control, _peer) = Control::pair().unwrap();
    control.completed = Some(Request::after(u64::MAX - 1, false).unwrap());
    assert_eq!(
        control.set_active(false).await.unwrap().sequence(),
        u64::MAX
    );
    assert!(control.set_active(true).await.is_err());
    assert!(control.set_active(false).await.is_err());
}

#[tokio::test]
async fn t1023_socket_lease_and_backpressure_preserve_pending_ownership() {
    let (mut control, peer) = Control::pair().unwrap();
    let socket = control.socket.get_ref().as_raw_fd();
    while packet::send(socket, &[1; RECORD_BYTES], &[]).is_ok() {}
    pending(control.set_active(false)).await;
    assert!(!control.pending.as_ref().unwrap().sent);
    let mut bytes = [0; RECORD_BYTES];
    while packet::receive(peer.0.as_raw_fd(), &mut bytes)
        .unwrap()
        .is_some()
    {}
    pending(control.set_active(true)).await;
    let original = command(&peer).await;
    assert!(!original.active());
    acknowledge(&peer, original);
    pending(control.set_active(true)).await;
    let next = command(&peer).await;
    assert!(next.active());
    acknowledge(&peer, next);
    assert_eq!(control.set_active(true).await.unwrap(), next);
    let lease = control.lease();
    drop(control);
    assert!(packet::send(peer.0.as_raw_fd(), &[1], &[]).is_ok());
    drop(lease);
    assert!(packet::send(peer.0.as_raw_fd(), &[1], &[]).is_err());
}

#[tokio::test]
async fn t1023_exec_inherits_only_reserved_endpoint() {
    let (control, peer) = Control::pair().unwrap();
    let parent = control.socket.get_ref().as_raw_fd();
    assert_ne!(
        unsafe { libc::fcntl(parent, libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let mut child = tokio::process::Command::new("/bin/sh");
    child.args(["-c", "test -S /proc/self/fd/\"$2\"", "control"]);
    peer.attach(&mut child).unwrap();
    assert!(child.status().await.unwrap().success());
    assert!(inherit(-1).is_err());
    let sentinel = tempfile::tempfile().unwrap();
    inherit(sentinel.as_raw_fd()).unwrap();
    assert_eq!(
        unsafe { libc::fcntl(sentinel.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
}

#[tokio::test]
async fn t1023_successful_transition_returns_original_proof() {
    let (mut control, peer) = Control::pair().unwrap();
    let responder = async {
        let request = command(&peer).await;
        acknowledge(&peer, request);
        request
    };
    let (result, proof) = tokio::join!(control.set_active(true), responder);
    assert_eq!(result.unwrap(), proof);
    assert_eq!(control.completed, Some(proof));
    assert!(control.pending.is_none());
}
