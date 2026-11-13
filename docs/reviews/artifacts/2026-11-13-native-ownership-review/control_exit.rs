use blent::control_workers::{Worker, WorkerPool};
use std::sync::mpsc;
fn main() {
    let pool = WorkerPool::new(1).unwrap();
    let (release, held) = mpsc::channel::<()>();
    let (entered_tx, entered) = mpsc::channel();
    let (proof_tx, proof_rx) = mpsc::channel();
    let (returned_tx, returned) = mpsc::channel();
    let original_pool = pool.clone();
    let original = std::thread::spawn(move || {
        blent::runtime_owner::run(async move {
            let worker = Worker::spawn(original_pool, "owned-held-control", move || {
                entered_tx.send(()).unwrap();
                let _ = held.recv();
            }).unwrap();
            proof_tx.send(worker.completion()).unwrap();
            drop(worker);
        }).unwrap();
        returned_tx.send(()).unwrap();
    });
    entered.recv().unwrap();
    let proof = proof_rx.recv().unwrap();
    let returned_early = returned.recv_timeout(std::time::Duration::from_secs(5)).is_ok();
    println!("process_boundary_returned_while_original_held={returned_early} native_complete={} pool_active={}", proof.is_complete(), pool.active());
    drop(release);
    original.join().unwrap();
    while !proof.is_complete() { std::thread::sleep(std::time::Duration::from_millis(1)); }
    println!("released_original native_complete={} pool_active={}", proof.is_complete(), pool.active());
}
