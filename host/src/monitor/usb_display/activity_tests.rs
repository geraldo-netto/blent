//! T1051: actual primary capture/selector ownership joins renewable USB policy.
use super::Event;
use super::*;
use blent_config::host_activity::*;
use std::sync::atomic::AtomicUsize;

struct Counted(Arc<AtomicUsize>);
impl blent_config::connection::registration_client::Factory for Counted {
    fn start(
        &mut self,
        request: blent_config::connection::registration_client::Request,
    ) -> Result<Box<dyn blent_config::connection::registration_client::Client>> {
        assert_eq!(
            request.resource,
            blent_config::connection::management::Resource::DisplaySession
        );
        self.0.fetch_add(1, Ordering::SeqCst);
        Registration.start(request)
    }
}
fn observation(locked: bool, revision: u64, feed: &mut Feed) -> Publication {
    feed.observe(blent_config::host_activity::Event {
        epoch: 1,
        revision,
        state: HostState {
            power: Some(Power::Running),
            locked: Some(locked),
            display_on: Some(!locked),
        },
    })
    .unwrap();
    Publication::Observed {
        capabilities: Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        snapshot: feed.snapshot(true, false),
    }
}
fn bind_primary(fixture: &mut Fixture) -> Arc<AtomicUsize> {
    fixture.links(&[(1, 2)]);
    let native = fixture.state.config.usb.as_mut().unwrap();
    native
        .connections
        .poll(&mut fixture.state.slots, Instant::now())
        .unwrap();
    let Decision::Start(admission) = native
        .connections
        .plan(&mut fixture.state.slots, Instant::now())
        .unwrap()
    else {
        panic!("T1051 primary admission missing")
    };
    let count = Arc::new(AtomicUsize::new(0));
    native.connections.bind(
        &mut fixture.state.slots,
        admission,
        Bindings {
            activity: Some(native.primary.activity.clone()),
            attachment: fixture.runtime.tablet_tx.clone(),
            registration: Box::new(Counted(count.clone())),
            entropy: blent_config::credentials::system_entropy,
            work: Box::new(RuntimeWork::shared(&fixture.runtime)),
        },
    );
    count
}
async fn active(fixture: &mut Fixture, now: Instant) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while fixture
            .state
            .config
            .usb
            .as_ref()
            .unwrap()
            .status()
            .is_empty()
        {
            let native = fixture.state.config.usb.as_mut().unwrap();
            native
                .connections
                .poll(&mut fixture.state.slots, now)
                .unwrap();
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1051_primary_native_pause_retains_capacity_and_configuration_through_fresh_registration()
{
    let (published, source) = watch::channel(Publication::Pending);
    let mut fixture = Fixture::with_activity(1, Some(source)).await;
    let count = bind_primary(&mut fixture);
    fixture.state.usb_progress();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let ticket = fixture
        .state
        .slots
        .ticket(&DisplayOwner::Peer(peer(2)))
        .unwrap();
    let mut feed = Feed::new(
        1,
        Capabilities {
            power: true,
            lock: true,
            display: true,
        },
        Settings::default(),
    )
    .unwrap();
    published.send_replace(observation(false, 1, &mut feed));
    active(&mut fixture, Instant::now()).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    fixture
        .settings
        .send_modify(|settings| settings.width = 1920);
    let old = fixture.runtime.tablet_tx.lease();
    published.send_replace(observation(true, 2, &mut feed));
    fixture.state.usb_progress();
    assert!(!old.apply(|| ()));
    assert!(fixture
        .state
        .config
        .usb
        .as_ref()
        .unwrap()
        .status()
        .is_empty());
    assert_eq!(fixture.state.slots.closing(ticket), Some(false));
    assert!(!*fixture.runtime.stop_tx.borrow());
    published.send_replace(observation(false, 3, &mut feed));
    active(&mut fixture, Instant::now() + Duration::from_secs(2)).await;
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert_eq!(
        fixture.state.slots.ticket(&DisplayOwner::Peer(peer(2))),
        Some(ticket)
    );
    assert_eq!(fixture.settings.borrow().width, 1920);
    fixture.links(&[]);
    fixture.state.usb_progress();
    assert!(fixture
        .state
        .config
        .usb
        .as_ref()
        .unwrap()
        .status()
        .is_empty());
    assert!(!*fixture.runtime.stop_tx.borrow());
    fixture.stop().await;
}

#[tokio::test]
async fn t1051_machine_publication_wakes_native_usb_without_polling_delay_or_closed_source_spin() {
    let (published, source) = watch::channel(Publication::Pending);
    let mut fixture = Fixture::with_activity(1, Some(source)).await;
    let mut interval = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(3600),
        Duration::from_secs(3600),
    );
    {
        let pending = event(&mut fixture.state.config.usb, &mut interval);
        tokio::pin!(pending);
        assert!(futures_util::poll!(pending.as_mut()).is_pending());
        published.send_replace(Publication::Failed);
        assert!(matches!(pending.await, Event::UsbTick));
    }
    drop(published);
    {
        let pending = event(&mut fixture.state.config.usb, &mut interval);
        tokio::pin!(pending);
        assert!(futures_util::poll!(pending.as_mut()).is_pending());
    }
    fixture.stop().await;
}

#[tokio::test]
async fn t1051_missing_original_native_capability_is_explicitly_unsupported() {
    let mut fixture = Fixture::new(1).await;
    let original = fixture.runtime.quiescence.take();
    assert!(activity_binding(&fixture.runtime, None).is_err());
    fixture.runtime.quiescence = original;
    assert!(activity_binding(&fixture.runtime, None).is_ok());
    fixture.stop().await;
}
