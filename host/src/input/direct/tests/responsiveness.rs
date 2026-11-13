use super::*;
use std::sync::Condvar;

#[derive(Default)]
struct Gate {
    entered: AtomicBool,
    released: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn wait(&self) {
        let mut released = self.released.lock().unwrap();
        self.entered.store(true, Ordering::Release);
        while !*released {
            released = self.changed.wait(released).unwrap();
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
struct HeldSave {
    evidence: Arc<Evidence>,
    gate: Arc<Gate>,
    entered: mpsc::Sender<()>,
}
impl Environment for HeldSave {
    type Device = Device;
    fn snapshot(&self) -> Result<Snapshot> {
        Fake(self.evidence.clone()).snapshot()
    }
    fn create(&self, mode: Mode) -> Result<Device> {
        Fake(self.evidence.clone()).create(mode)
    }
    fn save(&self, _: &Config) -> Result<()> {
        let _ = self.entered.send(());
        self.gate.wait();
        Ok(())
    }
}
async fn request(sink: &dyn InputSink) {
    sink.direct(Command::Negotiate { version: 1 })
        .unwrap()
        .await
        .unwrap();
}

#[test]
fn t1402_blocked_save_preserves_async_sibling_progress() {
    let gate = Arc::new(Gate::default());
    let original = gate.clone();
    let (entered, observe) = mpsc::channel();
    let (pulse, progress) = mpsc::channel();
    let backend = Backend::new(
        Config {
            monitor: "screen".into(),
            mode: Mode::DirectMouse,
        },
        true,
        true,
        move || HeldSave {
            evidence: Arc::new(Evidence::default()),
            gate: original,
            entered,
        },
    )
    .unwrap();
    let release = gate.clone();
    let watchdog = thread::spawn(move || {
        let entered = observe
            .recv_timeout(std::time::Duration::from_secs(3))
            .is_ok();
        let progressed = entered
            && progress
                .recv_timeout(std::time::Duration::from_millis(250))
                .is_ok();
        release.release(); // Always settle the original native call, including the faulty control.
        (entered, progressed)
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let sibling = tokio::spawn(async move {
            while !gate.entered.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
            let _ = pulse.send(());
        });
        request(&*backend.sink()).await;
        sibling.await.unwrap();
    });
    drop(backend);
    let (entered, progressed) = watchdog.join().unwrap();
    assert!(entered, "T1402 fixture never reached native save");
    assert!(
        progressed,
        "T1402 synchronous command wait blocked the async controller executor"
    );
}

struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}

fn held() -> (Backend, Arc<Evidence>, Arc<Gate>, mpsc::Receiver<()>) {
    let evidence = Arc::new(Evidence::default());
    let gate = Arc::new(Gate::default());
    let original = gate.clone();
    let events = evidence.clone();
    let (entered, observe) = mpsc::channel();
    let backend = Backend::new(
        Config {
            monitor: "screen".into(),
            mode: Mode::DirectMouse,
        },
        true,
        true,
        move || HeldSave {
            evidence: events,
            gate: original,
            entered,
        },
    )
    .unwrap();
    (backend, evidence, gate, observe)
}
async fn entered(observe: mpsc::Receiver<()>) {
    tokio::task::spawn_blocking(move || {
        observe
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
    })
    .await
    .unwrap();
}
pub(super) async fn retired(proof: blent_config::retirement::Completion) {
    tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        while !proof.is_complete() {
            assert!(
                started.elapsed() < std::time::Duration::from_secs(3),
                "T1402 worker did not retire"
            );
            thread::sleep(std::time::Duration::from_millis(1));
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn t1402_replacement_waits_for_original_worker_and_stale_reply_cannot_revoke_it() {
    let (backend, evidence, gate, observe) = held();
    let _release = Release(gate.clone());
    let sink = backend.sink();
    let original = sink.direct(Command::Negotiate { version: 1 }).unwrap();
    entered(observe).await;
    sink.release_all();
    assert!(!sink.direct_status().unwrap().negotiated);
    let replacement = sink.direct(Command::Negotiate { version: 1 }).unwrap();
    drop(original); // Its cancellation must not revoke the successor's authority.
    let allocations = evidence
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|call| call.starts_with("create"))
        .count();
    assert_eq!(
        allocations, 1,
        "T1402 replacement crossed original native save"
    );
    gate.release();
    replacement.await.unwrap();
    assert!(sink.direct_status().unwrap().negotiated);
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    retired(proof).await;
}

#[tokio::test]
async fn t1402_queue_rejects_overflow_and_cancellation_discards_unstarted_effects() {
    let (backend, evidence, gate, observe) = held();
    let _release = Release(gate.clone());
    let sink = backend.sink();
    let original = sink.direct(Command::Negotiate { version: 1 }).unwrap();
    entered(observe).await;
    let queued: Vec<_> = (0..16)
        .map(|_| sink.direct(mouse(Phase::Move, None, 0.5)).unwrap())
        .collect();
    assert!(sink.direct(mouse(Phase::Move, None, 0.5)).is_err());
    drop(queued); // Even unpolled receipts own cancellation, without blocking.
    assert!(original.await.is_err());
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    assert!(
        !proof.is_complete(),
        "T1402 caller loss forgot an original worker"
    );
    gate.release();
    retired(proof).await;
    assert!(!evidence
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| call.starts_with("mouse")));
}

#[tokio::test]
async fn t1402_controller_replacement_revokes_blocked_dispatch_promptly() {
    use super::super::super::{dispatch_controller_text, Controllers, SessionSettings};
    let (backend, _, gate, observe) = held();
    let _release = Release(gate.clone());
    let controllers = Arc::new(Controllers::new(backend.sink()));
    let first = controllers.claim();
    let (mode, _) = watch::channel(false);
    let settings = SessionSettings::new(&None, &mode, false);
    let latency = crate::latency::LatencyTracker::new();
    let pending = dispatch_controller_text(
        r#"{"type":"direct_input","command":{"type":"negotiate","version":1}}"#,
        &controllers,
        first.id,
        &settings,
        &latency,
        false,
    );
    entered(observe).await;
    let second = controllers.claim();
    assert!(
        !tokio::time::timeout(std::time::Duration::from_secs(1), pending.finish())
            .await
            .unwrap()
    );
    drop(first);
    drop(second);
    drop(controllers);
    let proof = backend.completion();
    drop(backend);
    assert!(!proof.is_complete());
    gate.release();
    retired(proof).await;
}

#[tokio::test]
async fn t1402_unpolled_reply_loss_revokes_status_without_native_wait() {
    let (backend, _) = backend(Mode::DirectMouse, true, true);
    let sink = backend.sink();
    negotiate(&*sink).await;
    assert!(sink.direct_status().unwrap().negotiated);
    let pending = sink.direct(mouse(Phase::Move, None, 0.5)).unwrap();
    drop(pending);
    assert!(!sink.direct_status().unwrap().negotiated);
    negotiate(&*sink).await;
    assert!(sink.direct_status().unwrap().negotiated);
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    retired(proof).await;
}

pub(super) async fn observed(evidence: &Evidence, prefix: &str, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let actual = evidence
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.starts_with(prefix))
                .count();
            if actual >= count {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("T1402 native effect not observed");
}

#[tokio::test]
async fn t1402_completed_unpolled_receipts_retain_bounded_admission() {
    let (backend, evidence) = backend(Mode::DirectMouse, true, true);
    let sink = backend.sink();
    negotiate(&*sink).await;
    let mut replies = Vec::new();
    for count in 1..=OUTSTANDING_COMMANDS {
        replies.push(sink.direct(mouse(Phase::Move, None, 0.5)).unwrap());
        observed(&evidence, "mouse", count).await;
    }
    assert!(sink.direct(mouse(Phase::Move, None, 0.5)).is_err());
    drop(replies); // Completed but unobserved work must still wake idle cleanup.
    observed(&evidence, "retire", 1).await;
    assert!(!sink.direct_status().unwrap().negotiated);
    negotiate(&*sink).await;
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    retired(proof).await;
}

#[tokio::test]
async fn t1402_retained_receipt_does_not_keep_closed_native_worker_alive() {
    let (backend, _, gate, observe) = held();
    let _release = Release(gate.clone());
    let pending = backend
        .sink()
        .direct(Command::Negotiate { version: 1 })
        .unwrap();
    entered(observe).await;
    let proof = backend.completion();
    drop(backend);
    assert!(!proof.is_complete());
    gate.release();
    retired(proof).await;
    assert!(pending.await.is_err());
}

mod session;
mod socket;
