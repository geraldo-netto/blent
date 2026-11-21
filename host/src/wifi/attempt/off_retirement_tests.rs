//! T1539: early off feedback never retires an unresolved original disconnect.
use super::*;
use crate::usb::{CommandFuture, RouteCommand, RouteFuture};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct State {
    acknowledge: bool,
    started: bool,
    recovered: AtomicBool,
    disconnects: AtomicUsize,
}

#[derive(Clone)]
struct Removal(Arc<State>);

impl Removal {
    fn inventory(&self) -> Result<std::process::Output> {
        if self.0.disconnects.load(Ordering::Acquire) == 0 {
            return Ok(crate::wifi::tests::output(
                0,
                "List of devices attached\nt1539-retire:5\tdevice\n",
            ));
        }
        ensure!(
            self.0.recovered.load(Ordering::Acquire),
            "injected unavailable inventory"
        );
        Ok(crate::wifi::tests::output(0, "List of devices attached\n"))
    }
}

impl Commands for Removal {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("T1539: fenced commands only")
    }
    fn execute_route(&self, args: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            let output = if args == ["devices"] {
                self.inventory()
            } else {
                assert_eq!(args, ["disconnect", "t1539-retire:5"]);
                self.0.disconnects.fetch_add(1, Ordering::AcqRel);
                if self.0.acknowledge {
                    Ok(crate::wifi::tests::output(0, "disconnected"))
                } else {
                    Err(anyhow::anyhow!("injected lost disconnect outcome"))
                }
            };
            RouteCommand {
                started: args[0] == "devices" || self.0.started,
                output,
            }
        })
    }
}

#[tokio::test]
async fn t1539_disconnect_keeps_exact_owner_until_inventory_recovers() {
    for (acknowledge, started) in [(false, false), (false, true), (true, true)] {
        let owners = Owners::new(1).unwrap();
        let command = Removal(Arc::new(State {
            acknowledge,
            started,
            recovered: AtomicBool::new(false),
            disconnects: AtomicUsize::new(0),
        }));
        let result = tokio::time::timeout(
            Duration::from_millis(300),
            Attempt::with_owner(&owners, Adb(command.clone()), "t1539-retire:5", false),
        )
        .await;
        let ready = matches!(&result, Ok(Ok(_)));
        let rejected = matches!(&result, Ok(Err(_)));
        drop(result);
        let active = owners.pool.active();
        let excluded = owners.reserve("t1539-retire:5").is_err();
        command.0.recovered.store(true, Ordering::Release);
        tests::quiet(&owners).await;
        assert_eq!(ready, acknowledge, "T1539: early caller feedback");
        assert_eq!(rejected, !started, "T1539: proven nonstart was retained");
        assert_eq!(
            active,
            usize::from(started),
            "T1539: original owner retention"
        );
        assert_eq!(excluded, started, "T1539: replacement admission");
        assert_eq!(command.0.disconnects.load(Ordering::Acquire), 1);
        assert!(owners.addresses.lock().unwrap().addresses.is_empty());
    }
}
