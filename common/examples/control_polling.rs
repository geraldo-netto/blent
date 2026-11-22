//! T1529 finite native observation workload; does not contact Blent services.
use blent_config::{commands::SyncCommandExt, retirement::Pool, spawn_reaped};
use std::{
    process::Command,
    time::{Duration, Instant},
};

fn retained() {
    let child = spawn_reaped(Command::new("sleep").arg("2")).unwrap();
    std::thread::sleep(Duration::from_millis(2100));
    while child.exit.borrow().is_none() {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(child.exit.borrow().unwrap().success());
}

fn command() {
    let output = Command::new("sh")
        .args(["-c", "printf begin; sleep 2; printf end"])
        .output_timeout(Duration::from_secs(5))
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"beginend");
}

fn fast() {
    let mut samples = Vec::with_capacity(50);
    for _ in 0..50 {
        let start = Instant::now();
        let output = Command::new("printf")
            .arg("hello")
            .output_bounded()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hello");
        samples.push(start.elapsed().as_micros());
    }
    samples.sort();
    println!("50 commands; p50={}us p95={}us", samples[25], samples[47]);
}

struct Broken;
impl blent_config::retirement::Observe for Broken {
    fn poll_complete(&mut self) -> bool {
        panic!("T1529 finite quarantine workload");
    }
}

fn quarantine() {
    let pool = Pool::new(1).unwrap();
    pool.reserve().unwrap().retain(Broken);
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(pool.active(), 1);
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("empty") => {
            let pool = Pool::new(1).unwrap();
            std::thread::sleep(Duration::from_secs(2));
            drop(pool);
        }
        Some("retained") => retained(),
        Some("command") => command(),
        Some("fast") => fast(),
        Some("quarantine") => quarantine(),
        _ => panic!("expected empty, retained, command, fast or quarantine"),
    }
}
