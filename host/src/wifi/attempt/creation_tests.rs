use super::*;
use crate::usb::CommandFuture;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[derive(Default)]
struct State {
    present: AtomicBool,
    entered: tokio::sync::Notify,
    reply: tokio::sync::Notify,
    removes: AtomicUsize,
}
#[derive(Clone, Default)]
struct Fake(Arc<State>);
impl Commands for Fake {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            match args[0].as_str() {
                "connect" => {
                    self.0.entered.notify_one();
                    self.0.reply.notified().await;
                    anyhow::bail!("T1364 lost creator acknowledgement")
                }
                "devices" => Ok(crate::wifi::tests::output(
                    0,
                    if self.0.present.load(Ordering::Acquire) {
                        "List of devices attached\nhost:5\tdevice\nunrelated:6\tdevice\n"
                    } else {
                        "List of devices attached\nunrelated:6\tdevice\n"
                    },
                )),
                "disconnect" => {
                    assert_eq!(args[1], "host:5");
                    self.0.removes.fetch_add(1, Ordering::AcqRel);
                    self.0.present.store(false, Ordering::Release);
                    Ok(crate::wifi::tests::output(0, "disconnected host:5"))
                }
                _ => panic!("unexpected command"),
            }
        })
    }
}
async fn lost_creation(effect_before_error: bool) {
    let owners = Owners::new(2).unwrap();
    let adb = Adb(Fake::default());
    let mut attempt = Box::pin(Attempt::with_owner(&owners, adb.clone(), "host:5", true));
    tokio::select! {
        _ = &mut attempt => panic!("held command returned"),
        _ = adb.0.0.entered.notified() => {}
    }
    drop(attempt);
    adb.0
         .0
        .present
        .store(effect_before_error, Ordering::Release);
    adb.0 .0.reply.notify_one();
    tokio::time::sleep(Duration::from_millis(40)).await;
    let retained = effect_before_error || owners.reserve("host:5").is_err();
    assert!(owners.reserve("other:5").is_ok());
    if !effect_before_error {
        assert_eq!(adb.0 .0.removes.load(Ordering::Acquire), 0);
        adb.0 .0.present.store(true, Ordering::Release);
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while owners.pool.active() != 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        retained,
        "T1364 unknown creator released address before its delayed effect"
    );
    assert_eq!(
        adb.0 .0.removes.load(Ordering::Acquire),
        1,
        "T1364 original creation leaked"
    );
    assert!(!adb.0 .0.present.load(Ordering::Acquire));
}
#[tokio::test]
async fn t1364_lost_acknowledgement_retains_delayed_creator() {
    lost_creation(false).await;
}
#[tokio::test]
async fn t1364_effect_before_error_is_compensated() {
    lost_creation(true).await;
}

struct Script(Mutex<std::collections::VecDeque<crate::usb::RouteCommand>>);
impl Commands for Script {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!()
    }
    fn execute_route(&self, _: Vec<String>) -> crate::usb::RouteFuture<'_> {
        Box::pin(async { self.0.lock().unwrap().pop_front().unwrap() })
    }
}
fn response(code: u32, text: &str) -> crate::usb::RouteCommand {
    crate::usb::RouteCommand {
        output: Ok(crate::wifi::tests::output(code, text)),
        started: true,
    }
}

#[tokio::test]
async fn t1544_canonical_reply_acknowledges_the_original_creator() {
    use blent_config::adb_reverse::temporary::Creation;
    let adb = Adb(Script(Mutex::new(
        [
            response(0, "List of devices attached\n"),
            response(0, "connected to LOCALHOST:55637\n"),
        ]
        .into(),
    )));
    let mut state = Creation::NotStarted;
    assert!(connect_route(&adb, "LOCALHOST:055637", &mut state)
        .await
        .unwrap());
    assert_eq!(state, Creation::Acknowledged);
    assert!(adb.0 .0.lock().unwrap().is_empty());
}
#[tokio::test]
async fn t1364_preexisting_and_unknown_inventory_never_start_creators() {
    use blent_config::adb_reverse::temporary::Creation;
    for (text, expected) in [
        ("List of devices attached\nhost:5\tdevice\n", true),
        ("List of devices attached\nhost:5\toffline\n", false),
        ("invalid", false),
    ] {
        let adb = Adb(Script(Mutex::new([response(0, text)].into())));
        let mut state = Creation::NotStarted;
        let result = connect_route(&adb, "host:5", &mut state).await;
        assert_eq!(result.is_ok(), expected);
        assert!(!result.unwrap_or(false));
        assert_eq!(state, Creation::NotStarted);
    }
}
#[tokio::test]
async fn t1364_invalid_replies_preserve_uncertainty_but_nonstarts_do_not() {
    use blent_config::adb_reverse::temporary::Creation;
    let no_start = crate::usb::RouteCommand {
        output: Err(anyhow::anyhow!("spawn failed")),
        started: false,
    };
    for command in [
        response(0, "invalid"),
        response(1, ""),
        response(1, "failed to connect to 'host:5': Connection refused"),
        response(0, "failed to connect to host:5"),
        response(0, "failed to authenticate to host:5"),
        response(0, &"x".repeat(65537)),
        no_start,
    ] {
        let expected = if command.started {
            Creation::Uncertain
        } else {
            Creation::NotStarted
        };
        let adb = Adb(Script(Mutex::new(
            [response(0, "List of devices attached\n"), command].into(),
        )));
        let mut state = Creation::NotStarted;
        assert!(connect_route(&adb, "host:5", &mut state).await.is_err());
        assert_eq!(state, expected);
    }
}

#[tokio::test]
async fn t1538_socket_refusal_settles_noncreation_but_not_an_unknown_connect() {
    use blent_config::adb_reverse::temporary::Creation;
    for text in [
        "failed to connect to 'host:5': Connection refused\n",
        "cannot connect to host:5: Connection refused\n",
    ] {
        let adb = Adb(Script(Mutex::new(
            [response(0, "List of devices attached\n"), response(0, text)].into(),
        )));
        let mut creation = Creation::NotStarted;
        assert!(connect_route(&adb, "host:5", &mut creation).await.is_err());
        assert_eq!(creation, Creation::NotStarted, "T1538: {text}");
        assert!(adb.0 .0.lock().unwrap().is_empty());
    }
}

#[derive(Default)]
struct RetryState {
    connects: AtomicUsize,
    inventories: AtomicUsize,
    present: AtomicBool,
}
#[derive(Clone, Default)]
struct RefusedThenReady(Arc<RetryState>);
impl Commands for RefusedThenReady {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            let text = match args[0].as_str() {
                "devices" => {
                    self.0.inventories.fetch_add(1, Ordering::AcqRel);
                    if self.0.present.load(Ordering::Acquire) {
                        "List of devices attached\nhost:5\tdevice\nunrelated:6\tdevice\n"
                    } else {
                        "List of devices attached\nunrelated:6\tdevice\n"
                    }
                }
                "connect" => {
                    assert_eq!(args[1], "host:5");
                    if self.0.connects.fetch_add(1, Ordering::AcqRel) == 0 {
                        "failed to connect to 'host:5': Connection refused\n"
                    } else {
                        self.0.present.store(true, Ordering::Release);
                        "connected to host:5\n"
                    }
                }
                "disconnect" => {
                    assert_eq!(args[1], "host:5");
                    self.0.present.store(false, Ordering::Release);
                    "disconnected host:5\n"
                }
                _ => panic!("unexpected command"),
            };
            Ok(crate::wifi::tests::output(0, text))
        })
    }
}

#[tokio::test]
async fn t1538_refused_attempt_releases_capacity_for_retry_and_process_exit() {
    let owners = Owners::new(1).unwrap();
    let adb = Adb(RefusedThenReady::default());
    let rejected = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        )
        .await,
        Ok(Err(_))
    );
    if !rejected {
        // Faulty-control cleanup: make the retained creator observable so its
        // original sponsor can settle before reporting the failed assertion.
        adb.0 .0.present.store(true, Ordering::Release);
    }
    tests::quiet(&owners).await;
    assert!(
        rejected,
        "T1538 explicit refusal retained an uncertain creator"
    );
    assert_eq!(adb.0 .0.inventories.load(Ordering::Acquire), 1);
    Attempt::with_owner(&owners, adb.clone(), "host:5", true)
        .await
        .unwrap()
        .finish(false)
        .await
        .unwrap();
    assert_eq!(adb.0 .0.connects.load(Ordering::Acquire), 2);
    assert!(!adb.0 .0.present.load(Ordering::Acquire));
    tests::quiet(&owners).await;
    shutdown(&owners);
    assert!(owners.reserve("host:5").is_err());
}
