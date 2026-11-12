use super::*;
use crate::persistence_port::{AsyncSnapshots, AsyncTransactions, Reply};
use blent_config::{persistence_contract::Edit, FileConfig};

struct ChangedAddress {
    current: Mutex<FileConfig>,
    later: &'static str,
}
impl AsyncSnapshots for ChangedAddress {
    fn snapshot(&self) -> Reply<'_> {
        // A snapshot captured before the concurrent update reaches its awaiter.
        Box::pin(async {
            Ok(FileConfig {
                wifi_address: "192.0.2.76:5555".into(),
                ..Default::default()
            })
        })
    }
}
impl AsyncTransactions for ChangedAddress {
    fn transact(&self, edit: Edit<'static>) -> Reply<'_> {
        Box::pin(async move {
            let mut current = self.current.lock().unwrap();
            edit(&mut current)?;
            let committed = current.clone();
            // A subsequent independent save must survive the earlier off call.
            current.wifi_address = self.later.into();
            Ok(committed)
        })
    }
}

#[tokio::test]
async fn t1377_disconnects_the_address_removed_by_the_transaction() {
    for later in ["", "192.0.2.78:5555"] {
        let (_root, _store, adb) = fixture();
        let store = ChangedAddress {
            current: Mutex::new(FileConfig {
                wifi_address: "192.0.2.77:5555".into(),
                ..Default::default()
            }),
            later,
        };
        adb.0
             .0
            .lock()
            .unwrap()
            .network
            .extend(["192.0.2.76:5555".into(), "192.0.2.77:5555".into()]);
        assert_eq!(
            policy::setup(&adb, &store, true).await.unwrap(),
            Setup::Disconnected
        );
        let state = adb.0 .0.lock().unwrap();
        assert!(
            state.network.contains("192.0.2.76:5555"),
            "T1377 disconnected the stale snapshot"
        );
        assert!(
            !state.network.contains("192.0.2.77:5555"),
            "T1377 left the cleared address connected"
        );
        assert_eq!(store.current.lock().unwrap().wifi_address, later);
    }
}

struct NoEdit;
impl AsyncSnapshots for NoEdit {
    fn snapshot(&self) -> Reply<'_> {
        panic!("T1377: off must read only inside the transaction")
    }
}
impl AsyncTransactions for NoEdit {
    fn transact(&self, _: Edit<'static>) -> Reply<'_> {
        Box::pin(async { Ok(FileConfig::default()) })
    }
}

#[tokio::test]
async fn t1377_missing_edit_and_failed_commit_cannot_admit_disconnect() {
    let (_root, _store, adb) = fixture();
    assert!(policy::setup(&adb, &NoEdit, true).await.is_err());
    assert!(policy::setup(&adb, &Failed, true).await.is_err());
    assert!(adb.0 .0.lock().unwrap().calls.is_empty());
}
