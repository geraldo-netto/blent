//! T1403 independent native-button model: bit 1 belongs to another input source.
use super::*;
use std::sync::atomic::AtomicUsize;

#[derive(Default)]
struct Record {
    release: AtomicBool,
    fail_save: AtomicBool,
    fail_event: AtomicBool,
    panic_event: AtomicBool,
    panic_cleanup: AtomicBool,
    attempts: AtomicUsize,
    creations: AtomicUsize,
    drops: AtomicUsize,
    unsafe_drops: AtomicUsize,
    buttons: AtomicUsize,
}
struct EnvironmentModel(Arc<Record>);
struct Native {
    record: Arc<Record>,
    owned: usize,
    settled: bool,
}
impl Adapter for Native {
    fn touch(&mut self, _: &[Contact]) -> Result<()> {
        anyhow::bail!("fixture has no touch device")
    }
    fn mouse(&mut self, _: (u16, u16), action: MouseAction) -> Result<()> {
        assert!(
            !self.record.panic_event.swap(false, Ordering::AcqRel),
            "T1403 native event panic"
        );
        ensure!(
            !self.record.fail_event.load(Ordering::Acquire),
            "native event rejected"
        );
        match action {
            MouseAction::Down(Button::Left) => {
                self.owned = 1;
                self.record.buttons.fetch_or(1, Ordering::AcqRel);
            }
            MouseAction::Up(Button::Left) => {
                self.owned = 0;
                self.record.buttons.fetch_and(!1, Ordering::AcqRel);
            }
            MouseAction::Move => (),
            _ => anyhow::bail!("fixture must preserve external buttons"),
        }
        Ok(())
    }
    fn retire(&mut self) -> Result<()> {
        self.record.attempts.fetch_add(1, Ordering::AcqRel);
        assert!(
            !self.record.panic_cleanup.swap(false, Ordering::AcqRel),
            "T1403 native cleanup panic"
        );
        ensure!(
            self.record.release.load(Ordering::Acquire),
            "native cleanup rejected"
        );
        self.record.buttons.fetch_and(!self.owned, Ordering::AcqRel);
        self.owned = 0;
        self.settled = true;
        Ok(())
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        if !self.settled {
            self.record.unsafe_drops.fetch_add(1, Ordering::AcqRel);
        }
        self.record.drops.fetch_add(1, Ordering::AcqRel);
    }
}
impl Environment for EnvironmentModel {
    type Device = Native;
    fn snapshot(&self) -> Result<Snapshot> {
        Fake(Arc::new(Evidence::default())).snapshot()
    }
    fn create(&self, _: Mode) -> Result<Native> {
        self.0.creations.fetch_add(1, Ordering::AcqRel);
        Ok(Native {
            record: self.0.clone(),
            owned: 0,
            settled: false,
        })
    }
    fn save(&self, _: &Config) -> Result<()> {
        ensure!(
            !self.0.fail_save.load(Ordering::Acquire),
            "configuration save failed"
        );
        Ok(())
    }
}
struct Recovery(Arc<Record>);
impl Drop for Recovery {
    fn drop(&mut self) {
        self.0.release.store(true, Ordering::Release);
    }
}
fn model() -> (Backend, Arc<Record>, Recovery) {
    let record = Arc::new(Record::default());
    record.buttons.store(2, Ordering::Release); // Unrelated physical right button.
    let native = record.clone();
    let backend = Backend::new(
        Config {
            monitor: "screen".into(),
            mode: Mode::DirectMouse,
        },
        true,
        true,
        move || EnvironmentModel(native),
    )
    .unwrap();
    (backend, record.clone(), Recovery(record))
}
async fn observe_cleanup(record: &Record) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while record.attempts.load(Ordering::Acquire) < 4
            && record.drops.load(Ordering::Acquire) == 0
        {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("T1403 no cleanup transition observed");
}
async fn press(sink: &dyn InputSink) {
    negotiate(sink).await;
    execute(sink, mouse(Phase::Down, Some(Button::Left), 0.5))
        .await
        .unwrap();
}
fn assert_settled(record: &Record, expected_devices: usize) {
    assert_eq!(
        record.buttons.load(Ordering::Acquire),
        2,
        "T1403 changed unrelated input or left its button held"
    );
    assert_eq!(record.drops.load(Ordering::Acquire), expected_devices);
    assert_eq!(
        record.unsafe_drops.load(Ordering::Acquire),
        0,
        "T1403 native owner dropped before release acceptance"
    );
}

#[tokio::test]
async fn t1403_caller_loss_retains_failed_button_cleanup_until_recovery() {
    let (backend, record, recovery) = model();
    press(&*backend.sink()).await;
    let proof = backend.completion();
    drop(backend);
    observe_cleanup(&record).await;
    let retained = !proof.is_complete() && record.drops.load(Ordering::Acquire) == 0;
    drop(recovery);
    responsiveness::retired(proof).await;
    assert!(
        retained,
        "T1403 caller loss discarded failed native button cleanup"
    );
    assert_settled(&record, 1);
}

#[tokio::test]
async fn t1403_failed_save_retains_provisional_native_owner() {
    let (backend, record, recovery) = model();
    record.fail_save.store(true, Ordering::Release);
    assert!(execute(&*backend.sink(), Command::Negotiate { version: 1 })
        .await
        .is_err());
    observe_cleanup(&record).await;
    let retained = record.drops.load(Ordering::Acquire) == 0;
    let proof = backend.completion();
    drop(backend);
    drop(recovery);
    responsiveness::retired(proof).await;
    assert!(
        retained,
        "T1403 failed save discarded provisional native owner"
    );
    assert_settled(&record, 1);
}

#[tokio::test]
async fn t1403_replacement_waits_for_failed_release_and_recovers() {
    let (backend, record, recovery) = model();
    let sink = backend.sink();
    press(&*sink).await;
    sink.release_all();
    let replacement = sink.direct(Command::Negotiate { version: 1 }).unwrap();
    observe_cleanup(&record).await;
    assert_eq!(record.creations.load(Ordering::Acquire), 1);
    assert!(sink.direct_status().unwrap().cleanup_pending);
    drop(recovery);
    replacement
        .await
        .expect("T1403 replacement abandoned while original release was pending");
    assert!(!sink.direct_status().unwrap().cleanup_pending);
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    responsiveness::retired(proof).await;
    assert_settled(&record, 2);
}

#[tokio::test]
async fn t1403_failed_event_returns_while_original_cleanup_remains_owned() {
    let (backend, record, recovery) = model();
    let sink = backend.sink();
    press(&*sink).await;
    record.fail_event.store(true, Ordering::Release);
    assert!(execute(&*sink, mouse(Phase::Move, None, 0.5))
        .await
        .is_err());
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    observe_cleanup(&record).await;
    let retained = !proof.is_complete() && record.drops.load(Ordering::Acquire) == 0;
    drop(recovery);
    responsiveness::retired(proof).await;
    assert!(retained, "T1403 failed event lost its cleanup sponsor");
    assert_settled(&record, 1);
}

#[tokio::test]
async fn t1403_adapter_unwinding_preserves_existing_native_obligations() {
    let (backend, record, recovery) = model();
    let sink = backend.sink();
    press(&*sink).await;
    record.panic_event.store(true, Ordering::Release);
    record.panic_cleanup.store(true, Ordering::Release);
    assert!(execute(&*sink, mouse(Phase::Move, None, 0.5))
        .await
        .is_err());
    observe_cleanup(&record).await;
    assert!(sink.direct_status().unwrap().cleanup_pending);
    assert_eq!(record.drops.load(Ordering::Acquire), 0);
    let proof = backend.completion();
    drop(sink);
    drop(backend);
    drop(recovery);
    responsiveness::retired(proof).await;
    assert_settled(&record, 1);
}
