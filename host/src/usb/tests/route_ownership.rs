//! T1344: independent delegated creator survives loss of its command waiter.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Default)]
struct Delegated {
    fake: Fake,
    admitted: Arc<Notify>,
    publish: Arc<Notify>,
    published: Arc<Notify>,
    first: Arc<AtomicBool>,
}
impl Commands for Delegated {
    fn execute(&self, arguments: Vec<String>, input: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            if arguments.iter().any(|v| v == "--no-rebind")
                && !self.first.swap(true, Ordering::AcqRel)
            {
                let owner = self.clone();
                let (done, received) = tokio::sync::oneshot::channel();
                tokio::spawn(async move {
                    owner.admitted.notify_one();
                    owner.publish.notified().await;
                    owner.fake.execute(arguments, input).await.unwrap();
                    owner.published.notify_one();
                    let _ = done.send(());
                });
                let _ = received.await;
                anyhow::bail!("T1344 lost creator acknowledgement");
            }
            self.fake.execute(arguments, input).await
        })
    }
}

async fn removed(fake: &Fake) -> bool {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if fake.0.lock().unwrap().routes.is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .is_ok()
}

#[tokio::test]
async fn t1344_lost_creation_acknowledgement_keeps_mapping_owned() {
    let commands = Delegated::default();
    commands.publish.notify_one();
    let adb = Adb(commands.clone());
    let mut routes = Routes::new("USB", (18000, 18001)).unwrap();
    assert!(routes.prepare(&adb).await.is_err());
    commands.published.notified().await; // Original effect occurred; cleanup may already have removed it.
    let _ = routes.retire(&adb).await;
    assert!(
        removed(&commands.fake).await,
        "T1344 lost acknowledgement abandoned its mapping"
    );
}

#[tokio::test]
async fn t1344_cancelled_waiter_and_owner_drop_retain_late_creator() {
    let commands = Delegated::default();
    let adb = Adb(commands.clone());
    let mut routes = Routes::new("USB", (18000, 18001)).unwrap();
    {
        let prepare = routes.prepare(&adb);
        tokio::pin!(prepare);
        tokio::select! {
            _ = commands.admitted.notified() => {},
            result = &mut prepare => panic!("creator must remain admitted: {result:?}"),
        }
    }
    drop(routes);
    commands.publish.notify_one();
    commands.published.notified().await;
    assert!(
        removed(&commands.fake).await,
        "T1344 late creator outlived its last owner"
    );
}

#[derive(Clone, Default)]
struct Unacknowledged(Fake);
impl Commands for Unacknowledged {
    fn execute(&self, arguments: Vec<String>, input: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            if arguments.iter().any(|value| value == "--no-rebind") {
                anyhow::bail!("T1344 original delegated outcome unknown");
            }
            self.0.execute(arguments, input).await
        })
    }
}
async fn complete(proof: blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("T1344 original retirement did not settle");
}

#[tokio::test]
async fn t1344_empty_snapshot_cannot_release_unacknowledged_creator() {
    let commands = Unacknowledged::default();
    let adb = Adb(commands.clone());
    let mut routes = Routes::new("USB", (18000, 18001)).unwrap();
    assert!(routes.prepare(&adb).await.is_err());
    let proof = routes.completion().unwrap();
    assert!(routes.retire(&adb).await.is_err());
    let premature = proof.is_complete();
    drop(routes);
    // Settle the original effect even if the assertion would fail.
    commands.0 .0.lock().unwrap().routes.insert(8890, 18000);
    complete(proof).await;
    assert!(
        !premature,
        "T1344 an empty snapshot is not creator settlement"
    );
    assert!(commands.0 .0.lock().unwrap().routes.is_empty());
}

#[tokio::test]
async fn t1344_caller_loss_cannot_delete_a_different_physical_tablet() {
    let fake = Fake::default();
    fake.0
        .lock()
        .unwrap()
        .identities
        .insert("USB".into(), "original".into());
    let adb = Adb(fake.clone());
    let mut routes = Routes::new("USB", (18000, 18001)).unwrap();
    routes.prepare(&adb).await.unwrap();
    let proof = routes.completion().unwrap();
    fake.0
        .lock()
        .unwrap()
        .identities
        .insert("USB".into(), "replacement".into());
    assert!(routes.retire(&adb).await.is_err());
    let preserved = fake.0.lock().unwrap().routes.len();
    let pending = !proof.is_complete();
    drop(routes);
    fake.0
        .lock()
        .unwrap()
        .identities
        .insert("USB".into(), "original".into());
    complete(proof).await;
    assert_eq!(preserved, 2);
    assert!(pending);
    assert!(fake.0.lock().unwrap().routes.is_empty());
}

#[tokio::test]
async fn t1344_network_without_physical_identity_fails_before_mutation() {
    let fake = Fake::default();
    let adb = Adb(fake.clone());
    let mut routes = Routes::for_transport("192.0.2.1:5555", (18000, 18001)).unwrap();
    assert!(routes.prepare(&adb).await.is_err());
    routes.retire(&adb).await.unwrap();
    assert!(!fake
        .0
        .lock()
        .unwrap()
        .calls
        .iter()
        .any(|(args, _)| args.iter().any(|arg| arg == "--no-rebind")));
}

#[tokio::test]
async fn t1540_failed_refresh_keeps_live_mappings_and_the_original_worker() {
    for failure in ["getprop", "reverse --list"] {
        let fake = Fake::default();
        fake.0
            .lock()
            .unwrap()
            .identities
            .insert("USB".into(), "original".into());
        let adb = Adb(fake.clone());
        let mut routes = Routes::new("USB", (18000, 18001)).unwrap();
        routes.prepare(&adb).await.unwrap();
        fake.0.lock().unwrap().fail = Some(failure.into());
        assert!(routes.prepare(&adb).await.is_err());
        fake.0.lock().unwrap().fail = None;
        let recovered = routes.prepare(&adb).await;
        let removed = fake
            .0
            .lock()
            .unwrap()
            .calls
            .iter()
            .any(|(args, _)| args.iter().any(|arg| arg == "--remove"));
        routes.retire(&adb).await.unwrap();
        assert!(
            recovered.is_ok(),
            "T1540 refresh lost its original worker: {recovered:?}"
        );
        assert!(
            !removed,
            "T1540 read-only refresh failure removed a live tunnel"
        );
        assert!(fake.0.lock().unwrap().routes.is_empty());
    }
}
