use super::*;
use crate::usb::CommandFuture;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[derive(Default)]
struct State {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    hold_connect: AtomicBool,
    hold_disconnect: AtomicBool,
    existing: AtomicBool,
    disconnected: AtomicUsize,
}
#[derive(Clone, Default)]
struct Fake(Arc<State>);
impl Commands for Fake {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            let connect = args[0] == "connect";
            if (connect && self.0.hold_connect.load(Ordering::Acquire))
                || (!connect && self.0.hold_disconnect.load(Ordering::Acquire))
            {
                self.0.entered.notify_one();
                self.0.release.notified().await;
            }
            if !connect {
                self.0.disconnected.fetch_add(1, Ordering::AcqRel);
            }
            let prefix = if self.0.existing.load(Ordering::Acquire) {
                "already connected to"
            } else {
                "connected to"
            };
            Ok(crate::wifi::tests::output(
                0,
                &format!("{prefix} {}", args[1]),
            ))
        })
    }
}
async fn quiet(owners: &Owners) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while owners.pool.active() != 0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn t834_cancel_during_connect_keeps_owner_until_ack_and_cleanup() {
    let owners = Owners::new(1).unwrap();
    let adb = Adb(Fake::default());
    adb.0 .0.hold_connect.store(true, Ordering::Release);
    let mut attempt = Box::pin(Attempt::with_owner(&owners, adb.clone(), "host:5", true));
    tokio::select! {
        _ = &mut attempt => panic!("held connect returned"),
        _ = adb.0.0.entered.notified() => {}
    }
    drop(attempt);
    assert_eq!(owners.pool.active(), 1);
    assert!(Attempt::with_owner(&owners, adb.clone(), "other:5", true)
        .await
        .is_err());
    adb.0 .0.release.notify_one();
    quiet(&owners).await;
    assert_eq!(adb.0 .0.disconnected.load(Ordering::Acquire), 1);
}
#[tokio::test]
async fn t834_old_cleanup_cannot_disconnect_new_attempt_or_stall_sibling() {
    let owners = Owners::new(2).unwrap();
    let adb = Adb(Fake::default());
    adb.0 .0.hold_disconnect.store(true, Ordering::Release);
    let attempt = Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        .await
        .unwrap();
    drop(attempt);
    adb.0 .0.entered.notified().await;
    assert!(Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        .await
        .is_err());
    Attempt::with_owner(&owners, adb.clone(), "other:5", true)
        .await
        .unwrap()
        .finish(true)
        .await
        .unwrap();
    assert_eq!(owners.pool.active(), 1);
    adb.0 .0.release.notify_one();
    quiet(&owners).await;
    Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        .await
        .unwrap()
        .finish(true)
        .await
        .unwrap();
    assert_eq!(adb.0 .0.disconnected.load(Ordering::Acquire), 1);
}
#[tokio::test]
async fn t834_cancellation_preserves_preexisting_route_and_off_is_owned() {
    let owners = Owners::new(1).unwrap();
    let adb = Adb(Fake::default());
    adb.0 .0.existing.store(true, Ordering::Release);
    drop(
        Attempt::with_owner(&owners, adb.clone(), "host:5", true)
            .await
            .unwrap(),
    );
    quiet(&owners).await;
    assert_eq!(adb.0 .0.disconnected.load(Ordering::Acquire), 0);
    Attempt::with_owner(&owners, adb.clone(), "host:5", false)
        .await
        .unwrap()
        .finish(true)
        .await
        .unwrap();
    assert_eq!(adb.0 .0.disconnected.load(Ordering::Acquire), 1);
}
#[test]
fn t834_invalid_addresses_and_capacity_do_not_admit_work() {
    for size in [0, 4097, usize::MAX] {
        assert!(Owners::new(size).is_err());
    }
    let owners = Owners::new(1).unwrap();
    for value in ["", "--all", "host:0", "host:65536", "host:5\n", "0.0.0.0:5"] {
        assert!(owners.reserve(value).is_err());
    }
    for byte in 0..=255 {
        let address = format!("{}:5", String::from_utf8_lossy(&[byte; 64]));
        let result = owners.reserve(&address);
        assert_eq!(result.is_ok(), crate::wifi::valid_address(&address));
        drop(result);
        assert_eq!(owners.pool.active(), 0);
    }
}

pub(crate) async fn quiet_global() {
    quiet(Owners::shared().unwrap()).await;
}

#[tokio::test]
async fn t834_optional_command_port_preserves_execution_and_lease_errors() {
    let some = Some(Fake::default());
    assert!(some.route_lease("host:5").is_ok());
    assert!(some
        .execute(vec!["connect".into(), "host:5".into()], None)
        .await
        .is_ok());
    let none = None::<Fake>;
    assert!(none.route_lease("host:5").is_err());
    assert!(none.execute(vec![], None).await.is_err());
}
