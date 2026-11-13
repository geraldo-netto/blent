//! Private-bus public API probe: one failed query for an existing session's UID.
use blent::{activity::PowerFactory, activity_follow::{Following, Settings}};
use blent_config::host_activity::Publication;
use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
const LOGIN: &str = "org.freedesktop.login1";
const PATH: &str = "/org/freedesktop/login1";
const SESSION: &str = "org.freedesktop.login1.Session";
const SESSION_PATH: &str = "/org/freedesktop/login1/session/owned";
unsafe extern "C" { fn geteuid() -> u32; }
struct Manager;
#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> bool { false }
    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> bool { false }
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> OwnedObjectPath {
        SESSION_PATH.try_into().unwrap()
    }
}
struct Session(Arc<AtomicUsize>);
#[zbus::interface(name = "org.freedesktop.login1.Session")]
impl Session {
    #[zbus(property)]
    fn locked_hint(&self) -> bool { true }
    #[zbus(property)]
    async fn user(&self) -> zbus::fdo::Result<(u32, OwnedObjectPath)> {
        if self.0.fetch_add(1, Ordering::AcqRel) == 0 {
            if std::env::args().any(|arg| arg == "timeout") {
                tokio::time::sleep(Duration::from_millis(200)).await;
            } else {
                return Err(zbus::fdo::Error::Failed("one transient User query failure".into()));
            }
        }
        Ok((unsafe { geteuid() }, "/org/freedesktop/login1/user/owned".try_into().unwrap()))
    }
}
async fn observed(reader: &mut tokio::sync::watch::Receiver<Publication>) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while matches!(*reader.borrow_and_update(), Publication::Pending) {
            reader.changed().await.unwrap();
        }
    }).await.unwrap();
}
async fn query(bus: &zbus::Connection, property: &str) -> OwnedValue {
    let reply = bus.call_method(Some(LOGIN), SESSION_PATH, Some("org.freedesktop.DBus.Properties"), "Get", &(SESSION, property)).await.unwrap();
    reply.body().deserialize().unwrap()
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
    println!("initial={:?}; identity_queries={}", *reader.borrow(), calls.load(Ordering::Acquire));
    tokio::time::sleep(Duration::from_millis(500)).await;
    println!("after_five_retry_intervals={:?}; identity_queries={}", *reader.borrow(), calls.load(Ordering::Acquire));
    println!("same_owner_fresh_user={:?}; process_uid={}; locked={:?}; publication={:?}", query(bus,"User").await, unsafe{geteuid()}, query(bus,"LockedHint").await, *reader.borrow());
    settled(following).await;
}
async fn run() {
    let calls = Arc::new(AtomicUsize::new(0));
    let server = zbus::connection::Builder::session().unwrap().name(LOGIN).unwrap().serve_at(PATH, Manager).unwrap().serve_at(SESSION_PATH, Session(calls.clone())).unwrap().build().await.unwrap();
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
