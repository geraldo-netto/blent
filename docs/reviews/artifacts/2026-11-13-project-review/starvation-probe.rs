use std::sync::{Arc,Mutex};
use std::time::Duration;
use blent::usb::{Adb,Commands,CommandFuture,args};
use blent_config::adb::{transport_of,Transport};
use blent::{transport,wifi};
use anyhow::{Result,ensure};
fn valid_serial(_: &str) -> bool { unreachable!("USB-only constructor unused by probe") }
#[path="/home/netto/Desktop/projects/blent/host/src/usb/routes.rs"]
mod routes;
use routes::Routes;
#[path="/home/netto/Desktop/projects/blent/host/src/monitor/launch_policy.rs"]
mod launch_policy;
#[path="/home/netto/Desktop/projects/blent/host/src/usb/network.rs"]
mod network;
#[derive(Clone)]
struct Fake(Arc<Mutex<usize>>);
impl Commands for Fake {
 fn execute(&self,args:Vec<String>,_:Option<Vec<u8>>) -> CommandFuture<'_> {
  Box::pin(async move {
   use std::os::unix::process::ExitStatusExt;
   let text=if args == ["devices"] { "List of devices attached\nslow-a\tdevice\nslow-b\tdevice\nhealthy\tdevice\n" }
    else { if args[1].starts_with("slow") { tokio::time::sleep(Duration::from_secs(4)).await; }
      else { *self.0.lock().unwrap() += 1; }
      "identity" };
   Ok(std::process::Output { status:std::process::ExitStatus::from_raw(0),stdout:text.as_bytes().to_vec(),stderr:vec![] })
  })
 }
}
#[tokio::main]
async fn main() {
 let root=tempfile::tempdir().unwrap();
 let mut network=network::Network::new(blent_config::storage::ConfigStore::new(root.path().join("config.toml")));
 let adb=Adb(Fake(Arc::new(Mutex::new(0))));
 for poll in 1..=2 {
  let result=tokio::time::timeout(Duration::from_secs(6),network.devices(&adb,&[])).await;
  println!("poll {poll}: completed inventory={} healthy identity queries={}",result.is_ok(),*adb.0.0.lock().unwrap());
 }
}
