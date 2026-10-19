impl LinuxSource {
    async fn on_buses(
        system: Option<zbus::Connection>,
        session: Option<zbus::Connection>,
        settings: Settings,
        session_id: Option<String>,
        desktop: &str,
    ) -> Result<Self> {
        settings.validate()?;
        let actors = Self::discover(system, session, settings, session_id, desktop).await;
        Self::start(fresh_epoch()?, settings, actors, lifetime())
    }
}
use super::*;
use blent_config::host_activity::{Policy, Power};
use std::sync::atomic::{AtomicI32, AtomicU8};
use zbus::{
    zvariant::{OwnedObjectPath, OwnedValue},
    Connection,
};
const LOGIN: &str = "org.freedesktop.login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";
const PATH: &str = "/org/freedesktop/login1";
const SESSION_PATH: &str = "/org/freedesktop/login1/session/fixture";
const DISPLAY: &str = "org.cinnamon.Muffin.DisplayConfig";
const DISPLAY_PATH: &str = "/org/cinnamon/Muffin/DisplayConfig";
#[derive(Default)]
pub(super) struct Values {
    sleep: AtomicBool,
    shutdown: AtomicBool,
    lock: AtomicBool,
    display: AtomicI32,
    mode: AtomicU8,
}
struct Manager(Arc<Values>);
#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> bool {
        self.0.sleep.load(Ordering::Acquire)
    }
    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> bool {
        self.0.shutdown.load(Ordering::Acquire)
    }
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> zbus::fdo::Result<OwnedObjectPath> {
        if self.0.mode.load(Ordering::Acquire) == 1 {
            return Err(zbus::fdo::Error::Failed("no process session".into()));
        }
        Ok(OwnedObjectPath::try_from(SESSION_PATH).unwrap())
    }
    fn get_session(&self, id: &str) -> zbus::fdo::Result<OwnedObjectPath> {
        if id != "fixture" {
            return Err(zbus::fdo::Error::Failed("unknown session".into()));
        }
        Ok(OwnedObjectPath::try_from(SESSION_PATH).unwrap())
    }
}
struct Session(Arc<Values>);
#[zbus::interface(name = "org.freedesktop.login1.Session")]
impl Session {
    #[zbus(property)]
    fn locked_hint(&self) -> bool {
        self.0.lock.load(Ordering::Acquire)
    }
    #[zbus(property)]
    fn user(&self) -> (u32, OwnedObjectPath) {
        let uid = unsafe { libc::geteuid() };
        let uid = if self.0.mode.load(Ordering::Acquire) == 2 {
            uid.wrapping_add(1)
        } else {
            uid
        };
        (
            uid,
            OwnedObjectPath::try_from("/org/freedesktop/login1/user/_fixture").unwrap(),
        )
    }
}
struct Display(Arc<Values>);
#[zbus::interface(name = "org.cinnamon.Muffin.DisplayConfig")]
impl Display {
    #[zbus(property)]
    fn power_save_mode(&self) -> i32 {
        self.0.display.load(Ordering::Acquire)
    }
}
pub(super) struct Fixture {
    pub(super) values: Arc<Values>,
    pub(super) server: Connection,
}
impl Fixture {
    pub(super) async fn new() -> Self {
        let values = Arc::new(Values::default());
        let server = zbus::connection::Builder::session()
            .unwrap()
            .name(LOGIN)
            .unwrap()
            .name(DISPLAY)
            .unwrap()
            .serve_at(PATH, Manager(values.clone()))
            .unwrap()
            .serve_at(SESSION_PATH, Session(values.clone()))
            .unwrap()
            .serve_at(DISPLAY_PATH, Display(values.clone()))
            .unwrap()
            .build()
            .await
            .unwrap();
        Self { values, server }
    }
    async fn source(&self, id: Option<&str>, desktop: &str, capacity: usize) -> LinuxSource {
        LinuxSource::on_buses(
            Some(Connection::session().await.unwrap()),
            Some(Connection::session().await.unwrap()),
            Settings {
                queue: capacity,
                ..Settings::default()
            },
            id.map(str::to_owned),
            desktop,
        )
        .await
        .unwrap()
    }
    async fn power(&self, member: &str, value: bool) {
        self.server
            .emit_signal(None::<&str>, PATH, MANAGER, member, &(value,))
            .await
            .unwrap();
    }
    async fn changed(
        &self,
        path: &str,
        interface: &str,
        property: &str,
        value: Option<OwnedValue>,
    ) {
        let mut values = std::collections::HashMap::new();
        let mut invalid = Vec::new();
        if let Some(value) = value {
            values.insert(property.to_owned(), value);
        } else {
            invalid.push(property.to_owned());
        }
        self.server
            .emit_signal(
                None::<&str>,
                path,
                "org.freedesktop.DBus.Properties",
                "PropertiesChanged",
                &(interface, values, invalid),
            )
            .await
            .unwrap();
    }
}
async fn event<S: Source + ?Sized>(source: &mut S) -> Event {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(event) = source.poll().unwrap() {
                return event;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap()
}
async fn retired<S: Source + ?Sized>(source: &S) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !source.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
pub(crate) fn private(test: &str) -> bool {
    if std::env::var_os("BLENT_T1008_PRIVATE_BUS").is_some() {
        std::env::set_var(
            "DBUS_SYSTEM_BUS_ADDRESS",
            std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),
        );
        return true;
    }
    let result = std::process::Command::new("dbus-run-session")
        .args(["--"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("BLENT_T1008_PRIVATE_BUS", "1")
        .env("XDG_CURRENT_DESKTOP", "Cinnamon")
        .env("XDG_SESSION_TYPE", "wayland")
        .env("XDG_SESSION_ID", "fixture")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    false
}
#[tokio::test]
async fn t1008_private_native_transitions_and_actual_source_retirement() {
    if !private("activity::tests::t1008_private_native_transitions_and_actual_source_retirement") {
        return;
    }
    let fixture = Fixture::new().await;
    let mut source = LinuxSource::connect(Settings::default(), lifetime())
        .await
        .unwrap();
    assert_eq!(
        source.capabilities(),
        Capabilities {
            power: true,
            lock: true,
            display: true
        }
    );
    let mut policy =
        Policy::new(source.epoch(), source.capabilities(), Default::default()).unwrap();
    policy.observe(event(&mut source).await).unwrap();
    assert!(policy.demand(true, false).display);
    assert!(!source.retired());
    for (member, value, expected) in [
        ("PrepareForSleep", true, Power::Suspending),
        ("PrepareForShutdown", true, Power::Unavailable),
        ("PrepareForSleep", false, Power::Unavailable),
        ("PrepareForShutdown", false, Power::Running),
    ] {
        fixture.power(member, value).await;
        if member == "PrepareForSleep" && !value {
            continue;
        }
        let observed = event(&mut source).await;
        assert_eq!(observed.state.power, Some(expected));
        policy.observe(observed).unwrap();
    }
    fixture
        .changed(SESSION_PATH, SESSION, "LockedHint", Some(true.into()))
        .await;
    policy.observe(event(&mut source).await).unwrap();
    assert!(!policy.demand(true, false).display);
    assert!(policy.demand(true, false).auxiliary);
    fixture
        .changed(SESSION_PATH, SESSION, "LockedHint", Some(false.into()))
        .await;
    policy.observe(event(&mut source).await).unwrap();
    fixture
        .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", Some(3i32.into()))
        .await;
    policy.observe(event(&mut source).await).unwrap();
    assert!(!policy.demand(true, false).display);
    assert!(policy.demand(true, false).auxiliary);
    fixture
        .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", Some(0i32.into()))
        .await;
    policy.observe(event(&mut source).await).unwrap();
    assert!(policy.demand(true, false).display);
    source.stop();
    source.stop();
    assert!(source.poll().unwrap().is_none());
    retired(&source).await;
}
#[tokio::test]
async fn t1008_private_owner_replacement_cannot_restore_old_source_or_pause_sibling_backend() {
    if !private("activity::tests::t1008_private_owner_replacement_cannot_restore_old_source_or_pause_sibling_backend") { return; }
    let fixture = Fixture::new().await;
    let mut source = fixture.source(None, "Cinnamon", 32).await;
    event(&mut source).await;
    fixture.server.release_name(DISPLAY).await.unwrap();
    let lost = event(&mut source).await;
    assert_eq!(lost.state.display_on, None);
    assert_eq!(lost.state.power, Some(Power::Running));
    let replacement = zbus::connection::Builder::session()
        .unwrap()
        .name(DISPLAY)
        .unwrap()
        .serve_at(DISPLAY_PATH, Display(fixture.values.clone()))
        .unwrap()
        .build()
        .await
        .unwrap();
    replacement
        .emit_signal(
            None::<&str>,
            DISPLAY_PATH,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &(
                DISPLAY,
                std::collections::HashMap::from([("PowerSaveMode", OwnedValue::from(0i32))]),
                Vec::<String>::new(),
            ),
        )
        .await
        .unwrap();
    fixture.power("PrepareForSleep", true).await;
    let next = event(&mut source).await;
    assert_eq!(next.state.display_on, None);
    assert_eq!(next.state.power, Some(Power::Suspending));
    let mut fresh = fixture.source(None, "Cinnamon", 32).await;
    assert!(fresh.epoch() > source.epoch());
    assert_eq!(event(&mut fresh).await.state.display_on, Some(true));
    source.stop();
    fresh.stop();
    retired(&source).await;
    retired(&fresh).await;
}
#[tokio::test]
async fn t1008_private_fallback_session_uid_and_capabilities_are_explicit() {
    if !private("activity::tests::t1008_private_fallback_session_uid_and_capabilities_are_explicit")
    {
        return;
    }
    let fixture = Fixture::new().await;
    fixture.values.mode.store(1, Ordering::Release);
    for id in [None, Some(""), Some("bad/id"), Some("unknown")] {
        let mut source = fixture.source(id, "KDE", 32).await;
        assert_eq!(
            source.capabilities(),
            Capabilities {
                power: true,
                lock: false,
                display: false
            }
        );
        source.stop();
        retired(&source).await;
    }
    let mut valid = fixture.source(Some("fixture"), "Cinnamon", 32).await;
    assert!(valid.capabilities().lock);
    valid.stop();
    retired(&valid).await;
    fixture.values.mode.store(2, Ordering::Release);
    fixture.values.display.store(-1, Ordering::Release);
    let mut other = fixture.source(None, "Cinnamon", 32).await;
    assert_eq!(
        other.capabilities(),
        Capabilities {
            power: true,
            lock: false,
            display: false
        }
    );
    other.stop();
    retired(&other).await;
}
#[tokio::test]
async fn t1008_unavailable_buses_and_invalid_settings_remain_explicit() {
    let mut source = LinuxSource::on_buses(None, None, Settings::default(), None, "unknown")
        .await
        .unwrap();
    assert_eq!(source.capabilities(), Capabilities::default());
    assert_eq!(event(&mut source).await.state, HostState::default());
    assert!(source.retired());
    source.stop();
    for deadline in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        assert!(LinuxSource::connect(
            Settings {
                deadline,
                ..Settings::default()
            },
            lifetime()
        )
        .await
        .is_err());
    }
    for queue in [0, 257, usize::MAX] {
        assert!(LinuxSource::connect(
            Settings {
                queue,
                ..Settings::default()
            },
            lifetime()
        )
        .await
        .is_err());
    }
    for start in [0, 1, u64::MAX - 1] {
        let counter = AtomicU64::new(start);
        assert_eq!(epoch(&counter).unwrap(), start + 1);
    }
    assert!(epoch(&AtomicU64::new(u64::MAX)).is_err());
}

#[tokio::test]
async fn t1008_private_invalidated_properties_require_fresh_original_snapshot() {
    if !private(
        "activity::tests::t1008_private_invalidated_properties_require_fresh_original_snapshot",
    ) {
        return;
    }
    let fixture = Fixture::new().await;
    let mut source = fixture.source(None, "Cinnamon", 32).await;
    event(&mut source).await;
    fixture.values.lock.store(true, Ordering::Release);
    fixture
        .changed(SESSION_PATH, SESSION, "LockedHint", None)
        .await;
    eprintln!("T1008 await lock invalidation");
    assert_eq!(event(&mut source).await.state.locked, None);
    eprintln!("T1008 await fresh lock snapshot");
    assert_eq!(event(&mut source).await.state.locked, Some(true));
    fixture.values.display.store(3, Ordering::Release);
    fixture
        .changed(DISPLAY_PATH, DISPLAY, "PowerSaveMode", None)
        .await;
    eprintln!("T1008 await display invalidation");
    assert_eq!(event(&mut source).await.state.display_on, None);
    eprintln!("T1008 await fresh display snapshot");
    assert_eq!(event(&mut source).await.state.display_on, Some(false));
    source.stop();
    retired(&source).await;
}

#[tokio::test]
async fn t1008_source_stop_keeps_actual_held_native_retirement() {
    let mut source = LinuxSource::on_buses(None, None, Settings::default(), None, "")
        .await
        .unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let (entered, entry) = tokio::sync::oneshot::channel();
    source.tasks.lock().unwrap().push(Work::asynchronous(
        Task::spawn_blocking(move || {
            entered.send(()).unwrap();
            blocked.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        })
        .unwrap(),
    ));
    entry.await.unwrap();
    source.stop();
    source.shared.update(native::Patch::Power(None)).unwrap(); // Late native callback cannot republish.
    assert!(source.poll().unwrap().is_none());
    assert!(!source.retired());
    release.send(()).unwrap();
    retired(&source).await;
}
#[tokio::test]
async fn t1008_private_missing_owners_are_unsupported_and_cannot_invent_observations() {
    if !private("activity::tests::t1008_private_missing_owners_are_unsupported_and_cannot_invent_observations") { return; }
    let mut source = LinuxSource::connect(Settings::default(), lifetime())
        .await
        .unwrap();
    assert_eq!(source.capabilities(), Capabilities::default());
    assert_eq!(event(&mut source).await.state, HostState::default());
    source.stop();
    retired(&source).await;
}

#[test]
fn t1008_compositor_properties_do_not_claim_direct_x11_display_power() {
    assert_eq!(compositor_desktop("wayland", "Cinnamon"), "Cinnamon");
    for session in ["", "x11", "tty", "Wayland", "unknown"] {
        assert_eq!(compositor_desktop(session, "Cinnamon"), "");
    }
}

async fn followed(
    reader: &mut tokio::sync::watch::Receiver<blent_config::host_activity::Publication>,
    predicate: impl Fn(blent_config::host_activity::ActivitySnapshot) -> bool,
) -> blent_config::host_activity::ActivitySnapshot {
    use blent_config::host_activity::Publication;
    let ready = tokio::time::timeout(
        Duration::from_secs(3),
        reader.wait_for(
            |value| matches!(value, Publication::Observed { snapshot, .. } if predicate(*snapshot)),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let Publication::Observed { snapshot, .. } = *ready else {
        unreachable!()
    };
    snapshot
}
#[tokio::test]
async fn t1018_private_native_factory_follows_configured_preferences_and_original_source_cleanup() {
    if !private("activity::tests::t1018_private_native_factory_follows_configured_preferences_and_original_source_cleanup") { return; }
    let fixture = Fixture::new().await;
    let following = crate::activity_follow::Following::start(
        Arc::new(Factory),
        crate::activity_follow::Settings {
            interval: Duration::from_millis(1),
            ..Default::default()
        },
        blent_config::host_activity::Settings {
            pause_on_lock: false,
            ..Default::default()
        },
    )
    .unwrap();
    let mut reader = following.subscribe();
    let first = followed(&mut reader, |snapshot| snapshot.demand.stamp.revision > 0).await;
    assert!(first.demand.display && first.demand.auxiliary);
    fixture
        .changed(SESSION_PATH, SESSION, "LockedHint", Some(true.into()))
        .await;
    let locked = followed(&mut reader, |snapshot| {
        snapshot.demand.stamp.revision > first.demand.stamp.revision
    })
    .await;
    assert!(locked.demand.display && locked.demand.auxiliary);
    fixture.power("PrepareForSleep", true).await;
    let asleep = followed(&mut reader, |snapshot| !snapshot.demand.auxiliary).await;
    assert!(!asleep.demand.display);
    fixture.power("PrepareForSleep", false).await;
    let resumed = followed(&mut reader, |snapshot| snapshot.demand.auxiliary).await;
    assert!(resumed.must_retire(
        blent_config::host_activity::ActivityTarget::Auxiliary,
        first.demand.stamp
    ));
    following.stop();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !following.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        *reader.borrow(),
        blent_config::host_activity::Publication::Stopped
    );
}

fn lifetime() -> SourceLifetime {
    SourceLifetime::new(
        blent_config::retirement::Pool::new(1)
            .unwrap()
            .reserve()
            .unwrap(),
    )
}

#[tokio::test]
async fn t1018_native_x11_unavailable_helper_reports_unsupported_without_cached_compositor_power() {
    if !private("activity::tests::t1018_native_x11_unavailable_helper_reports_unsupported_without_cached_compositor_power") { return; }
    // This isolated test executable lacks the daemon's private helper dispatcher.
    // Failed helper framing must not borrow a potentially stale X11 compositor hint.
    std::env::set_var("XDG_SESSION_TYPE", "x11");
    let fixture = Fixture::new().await;
    fixture.values.display.store(3, Ordering::Release);
    let following = crate::activity_follow::Following::start(
        Arc::new(Factory),
        crate::activity_follow::Settings {
            interval: Duration::from_millis(1),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    let mut reader = following.subscribe();
    let snapshot = followed(&mut reader, |snapshot| snapshot.demand.stamp.revision > 0).await;
    let blent_config::host_activity::Publication::Observed { capabilities, .. } = *reader.borrow()
    else {
        unreachable!()
    };
    assert!(capabilities.power && capabilities.lock);
    assert!(!capabilities.display);
    assert!(snapshot.demand.display && snapshot.demand.auxiliary);
    following.stop();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !following.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1084_private_power_factory_ignores_desktop_helpers_and_follows_original_logind() {
    use blent_config::host_activity::SourceFactory;
    if !private("activity::tests::t1084_private_power_factory_ignores_desktop_helpers_and_follows_original_logind") { return; }
    let fixture = Fixture::new().await;
    for session in ["wayland", "x11"] {
        std::env::set_var("XDG_SESSION_TYPE", session);
        let lifetime = lifetime();
        let proof = lifetime.completion();
        let mut original = PowerFactory
            .create(Settings::default(), lifetime)
            .await
            .unwrap();
        assert!(original.epoch > 0);
        assert_eq!(
            original.source.capabilities(),
            Capabilities {
                power: true,
                lock: true,
                display: false
            }
        );
        assert_eq!(
            event(&mut *original.source).await.state.power,
            Some(Power::Running)
        );
        for (member, value, expected) in [
            ("PrepareForSleep", true, Power::Suspending),
            ("PrepareForSleep", false, Power::Running),
            ("PrepareForShutdown", true, Power::Unavailable),
            ("PrepareForShutdown", false, Power::Running),
        ] {
            fixture.power(member, value).await;
            assert_eq!(
                event(&mut *original.source).await.state.power,
                Some(expected)
            );
        }
        original.source.stop();
        retired(&*original.source).await;
        drop(original);
        power_lifetime_released(proof).await;
    }
}

#[tokio::test]
async fn t1084_private_power_factory_reports_absent_logind_without_display_fallback() {
    use blent_config::host_activity::SourceFactory;
    if !private("activity::tests::t1084_private_power_factory_reports_absent_logind_without_display_fallback") { return; }
    let mut original = PowerFactory
        .create(Settings::default(), lifetime())
        .await
        .unwrap();
    assert_eq!(original.source.capabilities(), Capabilities::default());
    assert_eq!(
        event(&mut *original.source).await.state,
        HostState::default()
    );
    original.source.stop();
    retired(&*original.source).await;
}

async fn power_lifetime_released(proof: blent_config::retirement::Completion) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !proof.is_complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn invalid_power_factory(settings: Settings) {
    use blent_config::host_activity::SourceFactory;
    let original = lifetime();
    let proof = original.completion();
    assert!(PowerFactory.create(settings, original).await.is_err());
    power_lifetime_released(proof).await;
}
#[tokio::test]
async fn t1084_power_factory_invalid_settings_release_original_unstarted_admission() {
    for deadline in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        invalid_power_factory(Settings {
            deadline,
            ..Settings::default()
        })
        .await;
    }
    for queue in [0, 257, usize::MAX] {
        invalid_power_factory(Settings {
            queue,
            ..Settings::default()
        })
        .await;
    }
}
