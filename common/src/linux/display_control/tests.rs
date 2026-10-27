use super::*;

fn snapshot() -> crate::display_control::Snapshot {
    crate::display_control::Snapshot {
        owner: "a".repeat(64),
        assignments: vec![],
    }
}

#[test]
fn t1274_native_display_round_trip_and_owner_checks() {
    let (ready, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            assert!(Server::bind(0).is_err());
            assert!(Server::bind(65).is_err());
            let mut server = Server::bind(2).unwrap();
            ready.send(()).unwrap();
            let call = server.next().await.unwrap();
            assert!(matches!(call.command, Command::Inspect {}));
            server.respond(
                call,
                Reply {
                    snapshot: snapshot(),
                    error: None,
                },
            );
            // next drives response jobs while waiting for another command.
            assert!(
                tokio::time::timeout(Duration::from_millis(100), server.next())
                    .await
                    .is_err()
            );
            let (left, _right) = UnixStream::pair().unwrap();
            let uid = unsafe { libc::getuid() };
            peer(&left, uid, Some(std::process::id())).unwrap();
            assert!(peer(&left, uid.wrapping_add(1), None).is_err());
            assert!(peer(&left, uid, Some(std::process::id() + 1)).is_err());
            server.close().await;
            Server::bind(1).unwrap().close().await;
        });
    });
    receiver.recv().unwrap();
    let client = Client::new().unwrap();
    let reply = client
        .request(std::process::id(), &Command::Inspect {})
        .unwrap();
    assert_eq!(reply.snapshot, snapshot());
    assert!(client.request(u32::MAX, &Command::Inspect {}).is_err());
    worker.join().unwrap();
}

#[tokio::test]
async fn t1274_native_display_frames_are_bounded_and_validated() {
    for bytes in [
        vec![b'x'],
        vec![b'x'; MAX_FRAME + 1],
        b"{\"command\":\"inspect\",\"extra\":1}".to_vec(),
    ] {
        let (mut sender, stream) = UnixStream::pair().unwrap();
        let task = tokio::spawn(async move {
            sender.write_all(&bytes).await.unwrap();
            sender.shutdown().await.unwrap();
        });
        assert!(receive(stream, unsafe { libc::getuid() }).await.is_err());
        task.await.unwrap();
    }
    let (mut sender, _receiver) = UnixStream::pair().unwrap();
    assert!(write(&mut sender, &"x".repeat(MAX_FRAME)).await.is_err());
}
