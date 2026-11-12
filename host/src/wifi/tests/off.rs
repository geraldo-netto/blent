use super::*;

#[derive(Clone)]
struct UnavailableLease(&'static str);
impl Commands for UnavailableLease {
    fn route_lease(&self, _: &str) -> Result<Box<dyn Send>> {
        anyhow::bail!(self.0)
    }
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        panic!("T1374: command admitted without a route lease")
    }
}

#[tokio::test]
async fn t1374_failed_off_admission_is_reported_after_forgetting_address() {
    for reason in ["endpoint stripe busy", "native lease creation failed"] {
        let (_root, store, _) = fixture();
        store
            .update(|config| {
                config.wifi_address = "192.0.2.74:5555".into();
                Ok(())
            })
            .unwrap();
        let result = setup(&Adb(UnavailableLease(reason)), &store, true).await;
        assert!(
            result.is_err(),
            "T1374: failed off admission reported success"
        );
        assert!(store.load().wifi_address.is_empty());
    }
}

#[tokio::test]
async fn t1374_off_distinguishes_missing_configuration_and_missing_adapter() {
    let (_root, store, adb) = fixture();
    assert_eq!(
        setup(&adb, &store, true).await.unwrap(),
        Setup::NoSavedAddress
    );
    store
        .update(|config| {
            config.wifi_address = "192.0.2.74:5555".into();
            Ok(())
        })
        .unwrap();
    assert!(Some(adb.0).is_configured());
    let missing = Adb(None::<Fake>);
    assert!(!missing.0.is_configured());
    assert_eq!(
        setup(&missing, &store, true).await.unwrap(),
        Setup::AdbUnavailable
    );
    assert!(store.load().wifi_address.is_empty());
}
