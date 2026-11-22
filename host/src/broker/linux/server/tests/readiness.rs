use super::*;

#[tokio::test]
async fn t1778_pending_dispatcher_turns_do_not_allocate_per_client_futures() {
    let mut allocations = 0;
    for clients in [0, 1, 4, 16] {
        let mut fixture = Fixture::new().await;
        let mut sockets = Vec::new();
        for _ in 0..clients {
            sockets.push(fixture.client().await.1);
        }
        let mut context = std::task::Context::from_waker(futures_util::task::noop_waker_ref());
        // Initial registrations are outside the finite steady pending-turn scope.
        assert!(
            std::pin::pin!(std::future::poll_fn(|cx| fixture.server.readiness.poll(
                &mut fixture.server.clients,
                &mut fixture.server.links,
                cx
            )))
            .poll(&mut context)
            .is_pending()
        );
        let (_, counts) = crate::allocation_probe::measure(|| {
            for _ in 0..256 {
                let pending = std::future::poll_fn(|cx| {
                    fixture.server.readiness.poll(
                        &mut fixture.server.clients,
                        &mut fixture.server.links,
                        cx,
                    )
                });
                assert!(std::pin::pin!(pending).poll(&mut context).is_pending());
            }
        });
        drop(sockets);
        fixture
            .server
            .shutdown(Duration::from_secs(5))
            .await
            .unwrap();
        fixture.finish().await;
        println!(
            "T1778 clients={clients} turns=256 allocations={} reallocations={} requested_bytes={}",
            counts.allocations, counts.reallocations, counts.requested_bytes
        );
        allocations += counts.allocations + counts.reallocations;
    }
    assert_eq!(
        allocations, 0,
        "T1778 idle readiness allocated per-client futures"
    );
}
