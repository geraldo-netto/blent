//! T1655: retries slow down without discharging unknown endpoint effects.
use super::*;
use tokio::time::Instant;

struct Recorded {
    replies: Mutex<VecDeque<RouteCommand>>,
    calls: Mutex<Vec<(Instant, Vec<String>)>>,
}

impl Commands for Recorded {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("cleanup requires original command fences")
    }

    fn execute_route(&self, args: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            self.calls.lock().unwrap().push((Instant::now(), args));
            self.replies.lock().unwrap().pop_front().unwrap()
        })
    }
}

fn recorded(replies: Vec<RouteCommand>) -> Adb<Recorded> {
    Adb(Recorded {
        replies: Mutex::new(replies.into()),
        calls: Mutex::new(Vec::new()),
    })
}

fn verify(adb: &Adb<Recorded>, inventories: usize, removals: usize) {
    assert!(adb.0.replies.lock().unwrap().is_empty());
    let calls = adb.0.calls.lock().unwrap();
    let observed: Vec<_> = calls
        .iter()
        .filter(|(_, args)| args == &["devices"])
        .collect();
    assert_eq!(observed.len(), inventories);
    assert_eq!(calls.len(), inventories + removals);
    assert!(calls
        .iter()
        .all(|(_, args)| args == &["devices"] || args == &["disconnect", "host:5"]));
    let expected_ms = [100, 200, 400, 800, 1600, 3200, 5000, 5000, 5000, 5000];
    for (pair, expected) in observed.windows(2).zip(expected_ms) {
        let elapsed = pair[1].0.duration_since(pair[0].0);
        let expected = Duration::from_millis(expected);
        assert!(
            (expected..=expected + Duration::from_millis(1)).contains(&elapsed),
            "T1655: expected {expected:?}, observed {elapsed:?}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn t1655_inventory_failures_back_off_before_original_endpoint_removal() {
    let mut replies: Vec<_> = (0..8).map(|_| reply(1, "")).collect();
    replies.extend([
        listing("host:5\tdevice\n"),
        reply(0, "disconnected"),
        listing(""),
    ]);
    let adb = recorded(replies);
    retire(&adb, "host:5", Creation::Acknowledged).await;
    verify(&adb, 10, 1);
}

#[tokio::test(start_paused = true)]
async fn t1655_absence_retains_unknown_creator_and_late_removal_is_not_reissued() {
    let mut replies: Vec<_> = (0..8).map(|_| listing("")).collect();
    replies.extend([
        listing("host:5\tdevice\n"),
        reply(1, "unconfirmed disconnect"),
        listing("host:5\toffline\n"),
        listing(""),
    ]);
    let adb = recorded(replies);
    retire(&adb, "host:5", Creation::Uncertain).await;
    verify(&adb, 11, 1);
}

#[tokio::test(start_paused = true)]
async fn t1655_pending_removal_waits_for_absence_without_new_mutations() {
    let mut replies: Vec<_> = (0..8).map(|_| listing("host:5\toffline\n")).collect();
    replies.push(listing(""));
    let adb = recorded(replies);
    retire(&adb, "host:5", Creation::RemovalPending).await;
    verify(&adb, 9, 0);
}
