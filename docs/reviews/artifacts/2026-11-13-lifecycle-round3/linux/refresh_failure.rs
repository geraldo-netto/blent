//! Public API probe. Run only beneath a new private dbus-run-session.
use blent::{activity::Factory, activity_follow::{Following, Settings}};
use blent_config::host_activity::Publication;
use std::{sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}, time::Duration};
use tokio::sync::watch;
const NAME: &str = "org.cinnamon.Muffin.DisplayConfig";
const PATH: &str = "/org/cinnamon/Muffin/DisplayConfig";
#[derive(Default)]
struct Values { fail: AtomicBool, calls: AtomicUsize }
struct Display(Arc<Values>);
#[zbus::interface(name = "org.cinnamon.Muffin.DisplayConfig")]
impl Display {
    #[zbus(property)]
    fn power_save_mode(&self) -> zbus::fdo::Result<i32> {
        self.0.calls.fetch_add(1, Ordering::AcqRel);
        if self.0.fail.swap(false, Ordering::AcqRel) {
            return Err(zbus::fdo::Error::Failed("one transient query failure".into()));
        }
        Ok(0)
    }
}
async fn observed(reader: &mut watch::Receiver<Publication>, predicate: impl Fn(Publication) -> bool) -> Publication {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let value = *reader.borrow_and_update();
            if predicate(value) { return value; }
            reader.changed().await.unwrap();
        }
    }).await.unwrap()
}
fn running(value: Publication) -> bool {
    matches!(value, Publication::Observed { capabilities, snapshot } if capabilities.display && snapshot.demand.display)
}
fn paused(value: Publication) -> bool {
    matches!(value, Publication::Observed { capabilities, snapshot } if capabilities.display && !snapshot.demand.display)
}
async fn changed(bus: &zbus::Connection, invalid: bool) {
    let mut values = std::collections::HashMap::<&str,zbus::zvariant::OwnedValue>::new();
    let mut invalidated = Vec::new();
    if invalid { invalidated.push("PowerSaveMode"); } else { values.insert("PowerSaveMode", 0i32.into()); }
    bus.emit_signal(None::<&str>, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged", &(NAME, values, invalidated)).await.unwrap();
}
async fn settled(following: Following) {
    following.stop();
    while !following.retired() { tokio::time::sleep(Duration::from_millis(1)).await; }
}
async fn exercise(bus: &zbus::Connection, values: Arc<Values>) {
    let mut settings = Settings::default();
    settings.source.deadline = Duration::from_millis(100);
    settings.interval = Duration::from_millis(1);
    let following = Following::start(Arc::new(Factory::default()), settings, Default::default()).unwrap();
    let mut reader = following.subscribe();
    println!("initial={:?}", observed(&mut reader, running).await);
    values.fail.store(true, Ordering::Release);
    changed(bus, true).await;
    println!("invalidated={:?}", observed(&mut reader, paused).await);
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("after_five_retry_intervals={:?}; property_calls={}", *reader.borrow(), values.calls.load(Ordering::Acquire));
    let reply = bus.call_method(Some(NAME), PATH, Some("org.freedesktop.DBus.Properties"), "Get", &(NAME,"PowerSaveMode")).await.unwrap();
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().unwrap();
    println!("same_owner_fresh_query={value:?}; publication={:?}", *reader.borrow());
    changed(bus, false).await;
    println!("fresh_signal_control={:?}", observed(&mut reader, running).await);
    settled(following).await;
}
async fn run() {
    let values = Arc::new(Values::default());
    let server = zbus::connection::Builder::session().unwrap().name(NAME).unwrap().serve_at(PATH, Display(values.clone())).unwrap().build().await.unwrap();
    exercise(&server, values).await;
    server.close().await.unwrap();
    println!("following_and_server_settled=true");
}
fn main() {
    std::env::var("DBUS_SESSION_BUS_ADDRESS").expect("private bus required");
    std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/missing-round3-system-bus");
    std::env::set_var("XDG_SESSION_TYPE", "wayland");
    std::env::set_var("XDG_CURRENT_DESKTOP", "Cinnamon");
    blent::runtime_owner::run(run()).unwrap();
}
