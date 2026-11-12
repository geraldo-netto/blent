use super::*;
use crate::usb::CommandFuture;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct State {
    present: AtomicBool,
    remove_failed: tokio::sync::Notify,
    removes: AtomicUsize,
}
#[derive(Clone, Default)]
struct Fake(Arc<State>);
impl Commands for Fake {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            match args[0].as_str() {
                "connect" => {
                    self.0.present.store(true, Ordering::Release);
                    Ok(crate::wifi::tests::output(0, "connected to host:5"))
                }
                "disconnect" => {
                    self.0.removes.fetch_add(1, Ordering::AcqRel);
                    self.0.remove_failed.notify_one();
                    anyhow::bail!("injected lost removal acknowledgement")
                }
                "devices" => Ok(crate::wifi::tests::output(
                    0,
                    if self.0.present.load(Ordering::Acquire) {
                        "List of devices attached\nhost:5\toffline\nunrelated:6\tdevice\n"
                    } else {
                        "List of devices attached\nunrelated:6\tdevice\n"
                    },
                )),
                _ => panic!("unexpected command"),
            }
        })
    }
}

#[tokio::test]
async fn t1363_failed_rollback_retains_address_until_original_effect_is_observed() {
    let owners = Owners::new(2).unwrap();
    let adb = Adb(Fake::default());
    let attempt = Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        .await
        .unwrap();
    let completion = attempt.completion.clone();
    drop(attempt); // Caller loss still owns rollback.
    tokio::time::timeout(Duration::from_secs(2), adb.0 .0.remove_failed.notified())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    let retained = !completion.is_complete() && owners.reserve("host:5").is_err();
    let sibling = owners.reserve("other:5");
    assert!(sibling.is_ok());
    drop(sibling);
    adb.0 .0.present.store(false, Ordering::Release); // Original delayed removal settles.
    tokio::time::timeout(Duration::from_secs(2), async {
        while !completion.is_complete() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        retained,
        "T1363 failed rollback released its original address"
    );
    assert_eq!(
        adb.0 .0.removes.load(Ordering::Acquire),
        1,
        "T1363 uncertain removal was reissued"
    );
    assert!(owners.reserve("host:5").is_ok());
}

#[test]
fn t1363_reservations_share_canonical_endpoint_identity() {
    let owners = Owners::new(2).unwrap();
    for (first, alias) in [
        ("HOST:0005", "host:5"),
        ("[2001:0db8:0:0:0:0:0:1]:5", "[2001:db8::1]:5"),
    ] {
        let held = owners.reserve(first).unwrap();
        assert!(
            owners.reserve(alias).is_err(),
            "T1363 overlapping endpoint alias"
        );
        drop(held);
        assert!(owners.reserve(alias).is_ok());
    }
}
