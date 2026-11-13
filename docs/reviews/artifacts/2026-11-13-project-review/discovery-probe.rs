use std::sync::atomic::{AtomicUsize,Ordering};
#[path="/home/netto/Desktop/projects/blent/host/src/device_tasks.rs"]
mod device_tasks;
#[path="/home/netto/Desktop/projects/blent/host/src/discovery.rs"]
mod discovery;
static QUERIES: AtomicUsize = AtomicUsize::new(0);
static DEVICE: AtomicUsize = AtomicUsize::new(1);
async fn probe_device_identity(serial: &str, _: &str) -> Option<(String,String)> {
 QUERIES.fetch_add(1,Ordering::SeqCst);
 Some((serial.into(),format!("tablet-{}", DEVICE.load(Ordering::SeqCst))))
}
async fn app_presence_with(_: &str, _: &str) -> Option<bool> { Some(true) }
#[tokio::main]
async fn main() {
 let mut discovery=discovery::Discovery::new();
 let serial="192.0.2.1:5555";
 discovery.refresh(vec![serial.into()],"fake-adb");
 println!("first observation: {:?}",discovery.next().await);
 DEVICE.store(2,Ordering::SeqCst);
 discovery.refresh(vec![serial.into()],"fake-adb");
 println!("after endpoint replacement: {:?}",discovery.next().await);
 println!("identity query count across two observations: {}",QUERIES.load(Ordering::SeqCst));
 println!("actual identity now: {:?}",probe_device_identity(serial,"fake-adb").await);
 discovery.stop().await;
}
