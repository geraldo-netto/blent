use super::*;

#[tokio::test]
async fn t1771_empty_broker_never_admits_inventory_without_discovery_demand() {
    let mut fixture = Fixture::new().await;
    fixture.server.scan().unwrap();
    let admitted = fixture.server.inventory.is_some();
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    assert!(
        !admitted,
        "T1771 empty broker spawned an unneeded inventory helper"
    );
}

#[tokio::test]
async fn t1771_idle_cache_is_invalidated_before_the_next_clients_inspect() {
    let mut fixture = Fixture::new().await;
    fixture
        .server
        .inventory_result(Ok(vec![candidate()]))
        .unwrap();
    fixture.server.next_inventory = Instant::now() + Duration::from_secs(2);
    fixture.server.scan().unwrap();
    let cached = fixture.server.inventory_observed;
    let available = fixture.server.controller.inventory_available();
    let (client, remote) = fixture.client().await;
    fixture.server.inspect(client, 1).unwrap();
    let pending = fixture.server.clients[&client].inspection;
    fixture.server.scan().unwrap();
    let scheduled = fixture.server.inventory.is_some();
    drop(remote);
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    assert!(
        !cached && !available,
        "T1771 idle inventory remained advertised as current"
    );
    assert_eq!(
        pending,
        Some(1),
        "T1771 returning client got stale cached inventory"
    );
    assert!(
        scheduled,
        "T1771 returning demand did not trigger a fresh scan"
    );
}

async fn measured_empty_broker() -> usize {
    let mut fixture = Fixture::new().await;
    let helper = &fixture.server.settings.helper;
    let source = std::fs::read_to_string(helper).unwrap();
    std::fs::write(
        helper,
        source.replace(
            "import sys",
            "import sys\nwith open(__file__ + '.calls', 'a') as calls:\n    calls.write('scan\\n')",
        ),
    )
    .unwrap();
    // Finite observation window, not a deadline for a progressing workload.
    tokio::select! {
        result = fixture.server.run() => panic!("server exited: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(4200)) => (),
    }
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    let calls = std::fs::read_to_string(fixture.server.settings.helper.with_extension("calls"))
        .unwrap_or_default()
        .lines()
        .count();
    fixture.finish().await;
    calls
}

#[tokio::test]
async fn t1771_idle_observation_has_no_helper_process_spawns() {
    let calls = measured_empty_broker().await;
    println!("T1771 empty-broker helper spawns in 4200 ms: {calls}");
    assert_eq!(
        calls, 0,
        "T1771 empty broker performed unneeded native scans"
    );
}

#[tokio::test]
async fn t1771_last_client_leaving_keeps_admitted_inventory_until_native_retirement() {
    let mut fixture = Fixture::new().await;
    let (client, remote) = fixture.client().await;
    fixture.server.scan().unwrap();
    assert!(fixture.server.inventory.is_some());
    fixture.server.close_client(client).unwrap();
    drop(remote);
    bounded(async {
        while !fixture.server.clients.is_empty() {
            fixture.server.collect_clients();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    fixture.server.scan().unwrap();
    assert!(
        fixture.server.inventory.is_some(),
        "T1771 abandoned admitted helper"
    );
    assert!(!fixture.server.inventory.as_ref().unwrap().finished());
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    assert!(fixture.server.inventory.as_ref().unwrap().retired());
    fixture.server.scan().unwrap();
    assert!(
        fixture.server.inventory.is_none(),
        "T1771 retained or replaced retired idle helper"
    );
    fixture.finish().await;
}

#[tokio::test]
async fn t1771_saved_peer_keeps_discovery_running_without_a_window() {
    let mut fixture = Fixture::new().await;
    fixture.server.controller =
        Control::new(peer(0), vec![peer(8)], fixture.server.settings.control).unwrap();
    fixture.server.scan().unwrap();
    let admitted = fixture.server.inventory.is_some();
    fixture
        .server
        .shutdown(Duration::from_secs(5))
        .await
        .unwrap();
    fixture.finish().await;
    assert!(admitted, "T1771 remembered peer lost automatic discovery");
}
