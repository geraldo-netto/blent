//! T1539: failing an initial read creates no endpoint-cleanup obligation.
use super::*;
use crate::usb::{CommandFuture, RouteCommand, RouteFuture};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct State {
    recover: AtomicBool,
    reads: AtomicUsize,
    mode: u8,
}

#[derive(Clone)]
struct FailedInventory(Arc<State>);

fn failed(mode: u8) -> FailedInventory {
    FailedInventory(Arc::new(State {
        recover: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
        mode,
    }))
}

impl Commands for FailedInventory {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("T1539: use fenced route outcomes")
    }

    fn execute_route(&self, args: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            assert_eq!(args, ["devices"], "T1539: failed read admitted a mutation");
            self.0.reads.fetch_add(1, Ordering::AcqRel);
            if self.0.recover.load(Ordering::Acquire) {
                return RouteCommand {
                    started: true,
                    output: Ok(crate::wifi::tests::output(0, "List of devices attached\n")),
                };
            }
            let output = match self.0.mode {
                0 => Err(anyhow::anyhow!("injected missing ADB executable")),
                1 => Ok(crate::wifi::tests::output(1, "server unavailable")),
                _ => Ok(crate::wifi::tests::output(0, "malformed inventory")),
            };
            RouteCommand {
                started: self.0.mode != 0,
                output,
            }
        })
    }
}

#[tokio::test]
async fn t1539_initial_inventory_failure_releases_attempt_without_retry() {
    for mode in 0..3 {
        let owners = Owners::new(1).unwrap();
        let commands = failed(mode);
        let result = tokio::time::timeout(
            Duration::from_millis(300),
            Attempt::with_owner(&owners, Adb(commands.clone()), "t1539:5", false),
        )
        .await;
        let rejected = matches!(&result, Ok(Err(_)));
        drop(result);
        let reads = commands.0.reads.load(Ordering::Acquire);
        // Settle the original pre-fix owner before reporting the regression.
        commands.0.recover.store(true, Ordering::Release);
        tests::quiet(&owners).await;
        assert!(
            rejected,
            "T1539 mode={mode}: initial read failure stalled off"
        );
        assert_eq!(reads, 1, "T1539 mode={mode}: repeated an initial failure");
        assert!(owners.addresses.lock().unwrap().addresses.is_empty());
    }
}

#[tokio::test]
async fn t1539_off_presents_unavailability_and_preserves_forget_commit() {
    for mode in 0..3 {
        let directory = tempfile::tempdir().unwrap();
        let store = blent_config::storage::ConfigStore::new(directory.path().join("config.toml"));
        store
            .update(|config| {
                config.wifi_address = "t1539-policy:5".into();
                Ok(())
            })
            .unwrap();
        let result = crate::wifi::setup(&Adb(failed(mode)), &store, true).await;
        if mode == 0 {
            assert_eq!(result.unwrap(), crate::wifi::Setup::AdbUnavailable);
        } else {
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("transport disconnect unconfirmed"));
        }
        assert!(store.load().wifi_address.is_empty());
    }
    assert_eq!(Unavailable.to_string(), "ADB observation did not start");
}

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct MissingProgram(crate::usb::NativeCommands);

#[cfg(target_os = "linux")]
impl Commands for MissingProgram {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("T1539: native route outcomes only")
    }
    fn execute_route(&self, arguments: Vec<String>) -> RouteFuture<'_> {
        self.0.execute_route(arguments)
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn t1539_actual_missing_program_settles_without_endpoint_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let owners = Owners::new(1).unwrap();
    let adb = Adb(MissingProgram(crate::usb::NativeCommands(
        directory.path().join("missing-adb"),
    )));
    let error = Attempt::with_owner(&owners, adb, "t1539-native:5", false)
        .await
        .err()
        .unwrap();
    assert!(error.is::<Unavailable>());
    tests::quiet(&owners).await;
    assert!(owners.addresses.lock().unwrap().addresses.is_empty());
}
