use super::*;

async fn reports(remote: &AsyncSocket) -> Vec<Message> {
    let mut output = Vec::new();
    loop {
        let message = response(remote).await;
        let end = matches!(
            message,
            Message::Rejected { .. }
                | Message::Report {
                    notice: Notice::End { .. },
                    ..
                }
        );
        assert!(output.len() < 1602);
        output.push(message);
        if end {
            return output;
        }
    }
}

#[tokio::test]
async fn t1598_inventory_failure_keeps_saved_peers_and_pending_links_inspectable() {
    let mut fixture = Fixture::new().await;
    fixture.server.controller =
        Control::new(peer(0), vec![peer(8)], fixture.server.settings.control).unwrap();
    let (client, remote) = fixture.client().await;
    fixture
        .server
        .inventory_result(Ok(vec![candidate()]))
        .unwrap();
    let (revision, target) = fixture
        .server
        .controller
        .snapshot()
        .iter()
        .find_map(|notice| match notice {
            Notice::Device {
                revision, target, ..
            } => Some((*revision, *target)),
            _ => None,
        })
        .unwrap();
    let Action::Start { connection, .. } = fixture
        .server
        .controller
        .command(client, Command::Connect { revision, target }, 0)
        .unwrap()
    else {
        panic!("fixture admission");
    };
    let admission = fixture.server.link_pool.reserve().unwrap();
    fixture
        .server
        .controller
        .attach(connection, admission.completion())
        .unwrap();
    fixture
        .server
        .controller
        .identified(connection, peer(9), 1)
        .unwrap();
    fixture
        .server
        .inventory_result(Err(anyhow::anyhow!("unavailable")))
        .unwrap();
    control(&mut fixture.server, client, 1, Command::Inspect);
    let received = reports(&remote).await;
    drop(admission);
    fixture.finish().await;
    assert!(
        received.iter().any(|message| matches!(
            message,
            Message::Report {
                notice: Notice::Saved { .. },
                ..
            }
        )),
        "T1598 scan failure hid saved peers"
    );
    assert!(received.iter().any(|message| matches!(
        message,
        Message::Report {
            notice: Notice::Link {
                state: blent_config::connection::control::State::Pending,
                ..
            },
            ..
        }
    )));
}

#[tokio::test]
async fn t1598_first_inspect_waits_for_initial_inventory_outcome() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    control(&mut fixture.server, client, 1, Command::Inspect);
    let premature = tokio::time::timeout(Duration::from_millis(20), remote.receive()).await;
    fixture
        .server
        .inventory_result(Err(anyhow::anyhow!("first scan failed")))
        .unwrap();
    // A premature reply is enough to fail before requiring the future behavior.
    if premature.is_ok() {
        fixture.finish().await;
        panic!("T1598 first Inspect settled before inventory");
    }
    let received = reports(&remote).await;
    fixture.finish().await;
    assert!(matches!(
        received.first(),
        Some(Message::Report {
            notice: Notice::Begin { .. },
            ..
        })
    ));
}

#[tokio::test]
async fn t1598_only_one_initial_inspect_is_retained_per_client() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    control(&mut fixture.server, client, 1, Command::Inspect);
    control(&mut fixture.server, client, 2, Command::Inspect);
    let received = response(&remote).await;
    fixture.finish().await;
    assert!(
        matches!(
            received,
            Message::Rejected {
                request: 2,
                reason: Rejection::Busy
            }
        ),
        "T1598 expected bounded pending inspection"
    );
}

#[tokio::test]
async fn t1598_closed_client_discards_initial_inspection_without_reusing_it() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    control(&mut fixture.server, client, 1, Command::Inspect);
    assert_eq!(fixture.server.clients[&client].inspection, Some(1));
    fixture.server.close_client(client).unwrap();
    assert_eq!(fixture.server.clients[&client].inspection, None);
    fixture.server.inventory_result(Ok(vec![])).unwrap();
    assert!(bounded(remote.receive()).await.is_err());
    fixture.finish().await;
}

#[tokio::test]
async fn t1598_failed_inventory_rejects_connect_and_recovers_on_empty_success() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    fixture
        .server
        .inventory_result(Err(anyhow::anyhow!("scan failed")))
        .unwrap();
    control(
        &mut fixture.server,
        client,
        1,
        Command::Connect {
            revision: 1,
            target: 1,
        },
    );
    assert!(matches!(
        response(&remote).await,
        Message::Rejected {
            reason: Rejection::Unavailable,
            ..
        }
    ));
    fixture.server.inventory_result(Ok(vec![])).unwrap();
    control(&mut fixture.server, client, 2, Command::Inspect);
    let received = reports(&remote).await;
    fixture.finish().await;
    assert!(matches!(
        received.first(),
        Some(Message::Report {
            notice: Notice::Begin {
                inventory_available: true,
                ..
            },
            ..
        })
    ));
}
