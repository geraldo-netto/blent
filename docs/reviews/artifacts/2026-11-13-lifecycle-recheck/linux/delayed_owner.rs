//! Isolated public-API probe: run only under a new dbus-run-session instance.
use blent::{activity::PowerFactory, activity_follow::{Following, Settings}};
use blent_config::host_activity::Publication;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

const LOGIN: &str = "org.freedesktop.login1";
const PATH: &str = "/org/freedesktop/login1";
struct Manager;
#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    #[zbus(property)]
    fn preparing_for_sleep(&self) -> bool { false }
    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> bool { false }
    #[zbus(name = "GetSessionByPID")]
    fn get_session_by_pid(&self, _pid: u32) -> zbus::fdo::Result<zbus::zvariant::OwnedObjectPath> {
        Err(zbus::fdo::Error::Failed("private power-only fixture".into()))
    }
}

async fn server() -> zbus::Connection {
    zbus::connection::Builder::session().unwrap()
        .name(LOGIN).unwrap().serve_at(PATH, Manager).unwrap()
        .build().await.unwrap()
}

async fn observed(reader: &mut watch::Receiver<Publication>, predicate: impl Fn(Publication) -> bool) -> Publication {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let current = *reader.borrow_and_update();
            if predicate(current) { return current; }
            reader.changed().await.unwrap();
        }
    }).await.unwrap()
}

fn supported(publication: Publication) -> bool {
    matches!(publication, Publication::Observed { capabilities, snapshot }
        if capabilities.power && snapshot.demand.auxiliary)
}

fn unsupported(publication: Publication) -> bool {
    matches!(publication, Publication::Observed { capabilities, .. } if !capabilities.power)
}

async fn settled(following: Following) {
    following.stop();
    while !following.retired() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn run() {
    let original = server().await;
    let mut settings = Settings::default();
    settings.source.deadline = Duration::from_millis(100);
    settings.interval = Duration::from_millis(1);
    let following = Following::start(Arc::new(PowerFactory), settings, Default::default()).unwrap();
    let mut reader = following.subscribe();
    println!("initial={:?}", observed(&mut reader, supported).await);
    original.release_name(LOGIN).await.unwrap();
    println!("owner_absent_after_retry={:?}", observed(&mut reader, unsupported).await);
    let replacement = server().await;
    replacement.emit_signal(None::<&str>, PATH, LOGIN.to_string() + ".Manager",
        "PrepareForSleep", &(true,)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(350)).await;
    println!("replacement_present_after_suspend_signal={:?}", *reader.borrow());
    settled(following).await;
    replacement.close().await.unwrap();
    original.close().await.unwrap();
    println!("original_and_replacement_sources_closed=true");
}

fn main() {
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").expect("private bus required");
    std::env::set_var("DBUS_SYSTEM_BUS_ADDRESS", address);
    std::env::remove_var("XDG_SESSION_ID");
    blent::runtime_owner::run(run()).unwrap();
}
