use super::*;

async fn snapshot(remote: &AsyncSocket, request: u64) -> Option<Vec<Notice>> {
    remote
        .send(
            &Message::Control {
                request,
                command: Command::Inspect,
            },
            &[],
        )
        .await
        .unwrap();
    let mut notices = Vec::new();
    for _ in 0..1602 {
        let (actual, notice) = match response(remote).await {
            Message::Report { request, notice } => (request, notice),
            Message::Rejected {
                request: actual,
                reason: Rejection::Unavailable,
            } => {
                assert_eq!(actual, request);
                assert!(notices.is_empty());
                return None;
            }
            _ => panic!("snapshot failed"),
        };
        assert_eq!(actual, request);
        let end = matches!(notice, Notice::End { .. });
        notices.push(notice);
        if end {
            return Some(notices);
        }
    }
    panic!("unbounded snapshot");
}
async fn inventory(remote: &AsyncSocket) -> (u64, u64) {
    for request in 1..=100 {
        if let Some(pair) = snapshot(remote, request)
            .await
            .unwrap_or_default()
            .iter()
            .find_map(|notice| match notice {
                Notice::Device {
                    revision, target, ..
                } => Some((*revision, *target)),
                _ => None,
            })
        {
            return pair;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("helper inventory did not arrive");
}
#[tokio::test]
async fn t853_server_run_inspects_native_inventory_and_retires_failed_synthetic_link() {
    let mut fixture = Fixture::new().await;
    let remote = AsyncSocket::new(
        Socket::connect(
            &fixture.server.settings.runtime.join("broker.sock"),
            unsafe { libc::geteuid() },
        )
        .unwrap(),
    )
    .unwrap();
    let (stop, done) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        tokio::select! { result = fixture.server.run() => panic!("server failed: {result:?}"), _ = done => () }
        fixture
    });
    let (revision, target) = bounded(inventory(&remote)).await;
    remote
        .send(
            &Message::Control {
                request: 200,
                command: Command::Connect { revision, target },
            },
            &[],
        )
        .await
        .unwrap();
    let mut connection = None;
    for _ in 0..16 {
        match response(&remote).await {
            Message::Report {
                request: 200,
                notice: Notice::Link { connection: id, .. },
            } => connection = Some(id),
            Message::Report {
                request: 200,
                notice: Notice::End { .. },
            } => break,
            Message::Report { request: 200, .. } => (),
            _ => panic!("unexpected connect response"),
        }
    }
    assert!(connection.is_some());
    bounded(async {
        for request in 201..=400 {
            if !snapshot(&remote, request)
                .await
                .expect("inventory remained healthy")
                .iter()
                .any(|notice| matches!(notice, Notice::Link { .. }))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("failed helper still admitted");
    })
    .await;
    stop.send(()).unwrap();
    let fixture = bounded(running).await.unwrap();
    assert!(fixture.server.links.is_empty());
    assert_eq!(fixture.server.link_pool.active(), 0);
    fixture.finish().await;
}
