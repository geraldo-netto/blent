use blent_config::retirement::Pool;
use blent_config::retirement_work::Cleanup;
use std::{cell::RefCell, sync::mpsc, time::{Duration, Instant}};
struct NativeTail { entered: mpsc::Sender<()>, release: mpsc::Receiver<()>, settled: mpsc::Sender<()> }
impl Drop for NativeTail {
    fn drop(&mut self) {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        self.settled.send(()).unwrap();
    }
}
thread_local! { static TAIL: RefCell<Option<NativeTail>> = const { RefCell::new(None) }; }
fn main() {
    let pool = Pool::new(1).unwrap();
    let admission = pool.reserve().unwrap();
    let proof = admission.completion();
    let (entered, observe) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let (settled, settlement) = mpsc::channel();
    admission.retain(Cleanup::new(move || TAIL.with(|slot| {
        *slot.borrow_mut() = Some(NativeTail { entered, release: blocked, settled });
    })));
    observe.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !proof.is_complete() && Instant::now() < deadline { std::thread::yield_now(); }
    println!("completion_while_original_native_tls_blocked={}", proof.is_complete());
    println!("capacity_reused_while_original_native_tls_blocked={}", pool.reserve().is_ok());
    release.send(()).unwrap();
    settlement.recv_timeout(Duration::from_secs(5)).unwrap();
}
