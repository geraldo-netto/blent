use std::{sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}, time::Duration};
use blent::usb::{Adb, Commands, CommandFuture, RouteFuture, RouteCommand, args};
use blent_config::adb::{transport_of,Transport};
use blent::{device_tasks, transport, wifi, usb};
use anyhow::{ensure, Result};
fn valid_serial(_: &str) -> bool { unreachable!("USB-only constructor unused") }
#[path="/home/netto/Desktop/projects/blent/host/src/usb/routes.rs"]
mod routes;
use routes::Routes;
#[path="/home/netto/Desktop/projects/blent/host/src/monitor/launch_policy.rs"]
mod launch_policy;
#[path="network_body.inc"]
mod network;
#[derive(Clone, Default)]
struct Fake { present: Arc<AtomicBool>, queries: Arc<AtomicUsize> }
fn output(text: String) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;
    std::process::Output {status:std::process::ExitStatus::from_raw(0),stdout:text.into_bytes(),stderr:vec![]}
}
impl Commands for Fake {
    fn execute(&self,args:Vec<String>,_:Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            let text = match args[0].as_str() {
                "devices" => format!("List of devices attached\nUSB\tdevice\n{}",if self.present.load(Ordering::SeqCst) {"192.0.2.1:5555\tdevice\n"} else {""}),
                "connect" => {self.present.store(true,Ordering::SeqCst); "connected to 192.0.2.1:5555".into()},
                "disconnect" => {self.present.store(false,Ordering::SeqCst); String::new()},
                _ => {self.queries.fetch_add(1,Ordering::SeqCst); "healthy-tablet".into()},
            };
            Ok(output(text))
        })
    }
    fn execute_route(&self,args:Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            if args[0] != "disconnect" { tokio::time::sleep(Duration::from_secs(4)).await; }
            RouteCommand {output:self.execute(args,None).await,started:true}
        })
    }
}
#[tokio::main]
async fn main() {
    let exit = blent::wifi::ProcessExit;
    let root = tempfile::tempdir().unwrap();
    let store = blent_config::storage::ConfigStore::new(root.path().join("config.toml"));
    store.update(|config| {config.wifi_address="192.0.2.1:5555".into(); Ok(())}).unwrap();
    let adb=Adb(Fake::default());
    let mut network=network::Network::new(store);
    let first=tokio::time::timeout(Duration::from_secs(6),network.devices(&adb,&[])).await;
    println!("First confirmed USB inventory discarded by whole-poll deadline: {}",first.is_err());
    println!("Healthy USB identity queries after first poll: {}",adb.0.queries.load(Ordering::SeqCst));
    for index in 1..=3 {
        let next=network.devices(&adb,&[]).await;
        println!("Follow-up poll {index} while original attempt retires: {next:?}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    network.stop().await;
    drop(exit);
    println!("Original simulated network route removed: {}",!adb.0.present.load(Ordering::SeqCst));
}
