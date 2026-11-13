//! Isolated public API probe: one failed initial logind property query.
use blent::{activity::PowerFactory, activity_follow::{Following, Settings}};
use blent_config::host_activity::Publication;
use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
const LOGIN: &str = "org.freedesktop.login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const PATH: &str = "/org/freedesktop/login1";
struct Manager(Arc<AtomicUsize>);
#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> zbus::fdo::Result<bool> {
        if self.0.fetch_add(1, Ordering::AcqRel) == 0 {
            return Err(zbus::fdo::Error::Failed("one initial query failure".into()));
        }
        Ok(false)
    }
    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> bool { false }
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> zbus::fdo::Result<zbus::zvariant::OwnedObjectPath> {
        Err(zbus::fdo::Error::Failed("private power-only fixture".into()))
    }
}
async fn observed(reader: &mut tokio::sync::watch::Receiver<Publication>) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !matches!(*reader.borrow_and_update(), Publication::Observed { .. }) {
            reader.changed().await.unwrap();
        }
    }).await.unwrap();
}
async fn settled(following: Following) {
    following.stop();
    while !following.retired() { tokio::time::sleep(Duration::from_millis(1)).await; }
}
async fn exercise(bus: &zbus::Connection, calls: Arc<AtomicUsize>) {
    let mut settings = Settings::default();
    settings.source.deadline = Duration::from_millis(100);
    settings.interval = Duration::from_millis(1);
    let following = Following::start(Arc::new(PowerFactory::default()), settings, Default::default()).unwrap();
    let mut reader = following.subscribe();
    observed(&mut reader).await;
    println!("initial={:?}; property_calls={}", *reader.borrow(), calls.load(Ordering::Acquire));
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("after_five_retry_intervals={:?}; property_calls={}", *reader.borrow(), calls.load(Ordering::Acquire));
    let reply = bus.call_method(Some(LOGIN), PATH, Some("org.freedesktop.DBus.Properties"), "Get", &(MANAGER,"PreparingForSleep")).await.unwrap();
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize().unwrap();
    println!("same_owner_fresh_query={value:?}; publication={:?}", *reader.borrow());
    settled(following).await;
}
async fn run() {
    let calls = Arc::new(AtomicUsize::new(0));
    let server = zbus::connection::Builder::session().unwrap().name(LOGIN).unwrap().serve_at(PATH, Manager(calls.clone())).unwrap().build().await.unwrap();
    exercise(&server, calls).await;
    server.close().await.unwrap();
    println!("following_and_server_settled=true");
}
fn main() {
    let address=std::env::var("DBUS_SESSION_BUS_ADDRESS").expect("private bus required");
    std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", address);
    std::env::remove_var("XDG_SESSION_ID");
    blent::runtime_owner::run(run()).unwrap();
}
