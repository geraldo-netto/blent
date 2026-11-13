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
struct Fake(Arc<Mutex<bool>>);
impl Commands for Fake {
 fn execute(&self,args:Vec<String>,_:Option<Vec<u8>>) -> CommandFuture<'_> {
  Box::pin(async move {
   use std::os::unix::process::ExitStatusExt;
   let (code,text) = if args == ["devices"] { (0,"List of devices attached\n192.0.2.1:5555\tdevice\n") }
    else if *self.0.lock().unwrap() { (1,"") } else { (0,"original-tablet") };
   Ok(std::process::Output { status:std::process::ExitStatus::from_raw(code<<8),stdout:text.as_bytes().to_vec(),stderr:vec![] })
  })
 }
}
#[tokio::main]
async fn main() {
 let root=tempfile::tempdir().unwrap();
 let mut network=network::Network::new(blent_config::storage::ConfigStore::new(root.path().join("config.toml")));
 let adb=Adb(Fake(Arc::new(Mutex::new(false))));
 let serial="192.0.2.1:5555";
 network.devices(&adb,&[]).await.unwrap();
 let original=network.identity(serial).cloned();
 println!("initial identity: {original:?}");
 *adb.0.0.lock().unwrap()=true;
 network.devices(&adb,&[serial.into()]).await.unwrap();
 println!("fresh identity probe: {:?}",adb.identity(serial).await);
 println!("cached identity after failed probe: {:?}",network.identity(serial));
 let mut routes=Routes::for_transport(serial,(9000,9001)).unwrap();
 println!("can retire against unconfirmed endpoint: {}",network.can_retire(&adb,&mut routes,original.as_ref()).await);
}
