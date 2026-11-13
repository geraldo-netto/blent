//! T1413: native D-Bus loss must reach the follower's retained recovery path.
use super::*;
use blent_config::host_activity::{Publication, SourceCreation, SourceFactory, StartedSource};
use blent_config::retirement::{Completion, Pool};
use std::sync::atomic::AtomicUsize;

struct NativeFactory {
    calls: AtomicUsize,
    held: Mutex<Option<Completion>>,
    original: Mutex<Option<Completion>>,
    system: Mutex<Option<Connection>>,
}
impl SourceFactory for NativeFactory {
    fn create(&self, settings: Settings, lifetime: SourceLifetime) -> SourceCreation {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(held) = self.held.lock().unwrap().take() {
            lifetime.retain(held);
            *self.original.lock().unwrap() = Some(lifetime.completion());
        }
        let system = self.system.lock().unwrap().take();
        Box::pin(async move {
            let system = match system {
                Some(system) => system,
                None => Connection::session().await?,
            };
            let session = Connection::session().await?;
            let actors = LinuxSource::discover(
                Some(system),
                Some(session),
                settings,
                Some("fixture".into()),
                "Cinnamon",
            )
            .await;
            let source = LinuxSource::start(fresh_epoch()?, settings, actors, lifetime)?;
            Ok(StartedSource {
                epoch: source.epoch(),
                source: Box::new(source),
            })
        })
    }
}

async fn factory(held: Option<Completion>) -> (Arc<NativeFactory>, Connection) {
    let system = Connection::session().await.unwrap();
    (
        Arc::new(NativeFactory {
            calls: AtomicUsize::new(0),
            held: Mutex::new(held),
            original: Mutex::new(None),
            system: Mutex::new(Some(system.clone())),
        }),
        system,
    )
}

fn start(factory: Arc<NativeFactory>) -> crate::activity_follow::Following {
    crate::activity_follow::Following::start(
        factory,
        crate::activity_follow::Settings {
            source: Settings {
                deadline: Duration::from_millis(100),
                ..Default::default()
            },
            interval: Duration::from_millis(1),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap()
}

pub(super) async fn finish(following: crate::activity_follow::Following) {
    following.stop();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !following.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

pub(super) async fn replacement(fixture: &Fixture, name: &str) -> Connection {
    fixture.server.release_name(name).await.unwrap();
    zbus::connection::Builder::session()
        .unwrap()
        .name(name)
        .unwrap()
        .serve_at(PATH, Manager(fixture.values.clone()))
        .unwrap()
        .serve_at(SESSION_PATH, Session(fixture.values.clone()))
        .unwrap()
        .serve_at(DISPLAY_PATH, Display(fixture.values.clone()))
        .unwrap()
        .build()
        .await
        .unwrap()
}

async fn owner_recovery(name: &str) {
    let fixture = Fixture::new().await;
    let pool = Pool::new(1).unwrap();
    let held = pool.reserve().unwrap();
    let (factory, _system) = factory(Some(held.completion())).await;
    let following = start(factory.clone());
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |s| s.demand.display).await;
    let _replacement = replacement(&fixture, name).await;
    let lost = followed(&mut reader, |s| s.source_lost).await;
    assert!(
        !lost.demand.display,
        "T1413 loss must deny original display work"
    );
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(
        factory.calls.load(Ordering::Acquire),
        1,
        "T1413 original native fence"
    );
    assert!(!factory
        .original
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .is_complete());
    drop(held);
    let fresh = followed(&mut reader, |s| s.demand.display).await;
    assert!(fresh.demand.stamp.epoch > first.demand.stamp.epoch);
    assert!(factory
        .original
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .is_complete());
    assert_eq!(factory.calls.load(Ordering::Acquire), 2);
    stale_signal(&fixture, name).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        matches!(*reader.borrow(), Publication::Observed { snapshot, .. }
        if snapshot.demand.display && snapshot.demand.stamp.epoch == fresh.demand.stamp.epoch)
    );
    finish(following).await;
}

async fn stale_signal(fixture: &Fixture, name: &str) {
    if name == LOGIN {
        fixture.power("PrepareForSleep", true).await;
    } else {
        fixture
            .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", Some(3i32.into()))
            .await;
    }
}

#[tokio::test]
async fn t1413_logind_replacement_waits_for_original_native_settlement() {
    if !private(
        "activity::tests::recovery::t1413_logind_replacement_waits_for_original_native_settlement",
    ) {
        return;
    }
    owner_recovery(LOGIN).await;
}

#[tokio::test]
async fn t1413_compositor_replacement_waits_for_original_native_settlement() {
    if !private("activity::tests::recovery::t1413_compositor_replacement_waits_for_original_native_settlement") { return; }
    owner_recovery(DISPLAY).await;
}

#[tokio::test]
async fn t1413_native_transport_closure_recovers_with_fresh_epoch() {
    if !private(
        "activity::tests::recovery::t1413_native_transport_closure_recovers_with_fresh_epoch",
    ) {
        return;
    }
    let _fixture = Fixture::new().await;
    let (factory, system) = factory(None).await;
    let following = start(factory.clone());
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |s| s.demand.display).await;
    system.close().await.unwrap();
    let lost = followed(&mut reader, |s| s.source_lost).await;
    assert!(!lost.demand.display && !lost.demand.auxiliary);
    let fresh = followed(&mut reader, |s| s.demand.display).await;
    assert!(fresh.demand.stamp.epoch > first.demand.stamp.epoch);
    assert_eq!(factory.calls.load(Ordering::Acquire), 2);
    finish(following).await;
}

#[tokio::test]
async fn t1413_explicit_stop_never_recreates_native_sources() {
    if !private("activity::tests::recovery::t1413_explicit_stop_never_recreates_native_sources") {
        return;
    }
    let _fixture = Fixture::new().await;
    let (factory, _system) = factory(None).await;
    let following = start(factory.clone());
    let mut reader = following.subscribe();
    followed(&mut reader, |s| s.demand.display).await;
    finish(following).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(*reader.borrow(), Publication::Stopped);
    assert_eq!(factory.calls.load(Ordering::Acquire), 1);
}
