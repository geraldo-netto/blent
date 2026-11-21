use blent_config::camera::{CameraBackend, CameraProfile, CameraState as State};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
#[path = "../../../../../../gui/src/media_owner.rs"]
mod media_owner;

fn failed_restart() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let (begun, started) = std::sync::mpsc::channel();
    let backend = blent::camera_control::Controller::with_pool(
        blent::control_workers::WorkerPool::new(1).unwrap(),
        move |_, mut stop, _| {
            count.fetch_add(1, Ordering::SeqCst);
            begun.send(()).unwrap();
            async move {
                blent::camera_control::cancelled(&mut stop).await;
                anyhow::bail!("original cleanup failed")
            }
        },
    )
    .unwrap();
    backend.start(CameraProfile::default()).unwrap();
    started.recv().unwrap();
    let requested = backend.start(CameraProfile::default()).unwrap();
    let mut owner = media_owner::Owner::default();
    owner.admit(None, requested.clone());
    while !matches!(backend.state(), State::Failed(_)) {
        std::thread::yield_now();
    }
    println!(
        "restart: native={:?}, receipt_pending={}, GUI={:?}, sessions_started={}",
        backend.state(),
        requested.is_pending(),
        owner.activity(|| blent_config::workspace::Activity::Failed),
        calls.load(Ordering::SeqCst)
    );
    let retirement = backend.shutdown();
    while !retirement.is_complete() {
        std::thread::yield_now();
    }
    println!(
        "restart: original worker retired; second receipt still pending={}",
        requested.is_pending()
    );
}
fn main() {
    failed_restart();
}
