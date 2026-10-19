use super::*;
use crate::{accessory::Owner, secure_connection::IdentityRecord};
use blent_config::retirement::{Admission, Pool};
use std::{future::Future, pin::Pin};

struct Native(Admission);
impl Owner for Native {
    fn completion(&self) -> Completion {
        self.0.completion()
    }
    fn retire(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}
fn resource() -> (Resource, tokio::io::DuplexStream) {
    let (stream, peer) = tokio::io::duplex(8192);
    (
        Resource {
            stream: Box::new(stream),
            owner: Box::new(Native(Pool::new(1).unwrap().reserve().unwrap())),
        },
        peer,
    )
}
fn settings() -> Settings {
    Settings {
        resource_timeout: AdmissionTimeout::default(),
        tls_timeout: blent_config::connection::handshake::Timeout::default(),
        identity: Arc::new(Identity::load(IdentityRecord::generate().unwrap()).unwrap()),
        limits: Limits::default(),
        block: 65536,
        setup_timeout: Duration::from_secs(1),
        approval_timeout: Duration::from_secs(120),
        origin: Instant::now(),
    }
}
#[tokio::test]
async fn t853_cancelled_discovery_drops_task_and_native_resource() {
    let (resource, _peer) = resource();
    let mut handle = Handle::start(resource, settings()).unwrap();
    let native = handle.native_completion();
    let task = handle.task_completion();
    assert!(!native.is_complete());
    handle.stop();
    assert!(tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .unwrap()
        .is_err());
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), async {
        while !task.is_complete() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(native.is_complete());
}
#[tokio::test]
async fn t853_invalid_setup_fails_without_native_leak() {
    let (resource, _peer) = resource();
    let mut options = settings();
    options.setup_timeout = Duration::ZERO;
    let mut handle = Handle::start(resource, options).unwrap();
    let native = handle.native_completion();
    assert!(tokio::time::timeout(Duration::from_secs(2), handle.next())
        .await
        .unwrap()
        .is_err());
    assert!(native.is_complete());
    assert!(now(Instant::now()).unwrap() < 1000);
    let (out, _input) = mpsc::channel(1);
    let peer = PeerId::parse("a".repeat(64)).unwrap();
    emit(&out, Event::Identified(peer.clone())).unwrap();
    assert!(emit(&out, Event::Identified(peer)).is_err());
}

#[tokio::test]
async fn t853_repeated_link_next_after_task_completion_does_not_panic() {
    use futures_util::FutureExt;
    let (resource, _peer) = resource();
    let mut options = settings();
    options.setup_timeout = Duration::ZERO;
    let mut handle = Handle::start(resource, options).unwrap();
    let (_keep_open, receiver) = mpsc::channel(1);
    handle.events = receiver;
    assert!(tokio::time::timeout(Duration::from_secs(3), handle.next())
        .await
        .unwrap()
        .is_err());
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        std::panic::AssertUnwindSafe(handle.next()).catch_unwind(),
    )
    .await
    .unwrap();
    assert!(result.is_ok(), "completed task was polled again");
    assert!(result.unwrap().is_err());
}

mod factory;
mod handshake;

mod timeout;
