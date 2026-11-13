use blent::activity_follow::Following;
use blent_config::{
    host_activity::*,
    retirement::{Completion, Pool},
};
use std::{
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
struct Factory {
    release: Mutex<Option<mpsc::Receiver<()>>>,
    native: Mutex<Option<Completion>>,
    mode: String,
}
struct SourceOwner {
    native: Completion,
    _lifetime: SourceLifetime,
}
impl Source for SourceOwner {
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn poll(&mut self) -> anyhow::Result<Option<Event>> {
        Ok(None)
    }
    fn stop(&mut self) {}
    fn retired(&self) -> bool {
        self.native.is_complete()
    }
}
impl SourceFactory for Factory {
    fn create(&self, _: SourceSettings, lifetime: SourceLifetime) -> SourceCreation {
        let hold = self.release.lock().unwrap().take().unwrap();
        let admission = Pool::new(1).unwrap().reserve().unwrap();
        let native = admission.completion();
        lifetime.retain(native.clone());
        let thread = std::thread::spawn(move || {
            hold.recv().unwrap();
        });
        admission.retain(blent_config::retirement_work::Thread::new(thread));
        *self.native.lock().unwrap() = Some(native.clone());
        let source = SourceOwner {
            native,
            _lifetime: lifetime,
        };
        creation(self.mode.clone(), source)
    }
}
fn creation(mode: String, source: SourceOwner) -> SourceCreation {
    Box::pin(async move {
        match mode.as_str() {
            "cancel" => std::future::pending().await,
            "error" => anyhow::bail!("T1421 source creation failed"),
            _ => Ok(StartedSource {
                epoch: 1,
                source: Box::new(source),
            }),
        }
    })
}
async fn settle(source: &Following) {
    source.stop();
    while !source.retired() {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
async fn work(
    factory: Arc<Factory>,
    staged: mpsc::Sender<()>,
    release: mpsc::Sender<()>,
) -> Result<(), &'static str> {
    let source = Following::start(factory.clone(), Default::default(), Default::default()).unwrap();
    while factory.native.lock().unwrap().is_none() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    if factory.mode == "normal" {
        release.send(()).unwrap();
        settle(&source).await;
    }
    if factory.mode == "delayed" {
        assert!(
            tokio::time::timeout(Duration::from_secs(5), settle(&source))
                .await
                .is_err()
        );
    }
    staged.send(()).unwrap();
    match factory.mode.as_str() {
        "panic" => panic!("T1421 work panic"),
        "error" => Err("T1421 work error"),
        _ => Ok(()),
    }
}
pub(super) fn run(mode: &str) {
    let (release, hold) = mpsc::channel();
    let factory = Arc::new(Factory {
        release: Mutex::new(Some(hold)),
        native: Mutex::new(None),
        mode: mode.into(),
    });
    let (staged, ready) = mpsc::channel();
    let (done, returned) = mpsc::channel();
    let owned = factory.clone();
    let native_release = release.clone();
    let worker = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| {
            blent::runtime_owner::run(work(owned, staged, native_release))
        });
        done.send(result).unwrap();
    });
    ready.recv_timeout(Duration::from_secs(8)).unwrap();
    let native = factory.native.lock().unwrap().clone().unwrap();
    let early = returned.recv_timeout(Duration::from_millis(200));
    let early_return = early.is_ok();
    if mode != "normal" {
        assert!(!native.is_complete());
        release.send(()).unwrap();
    }
    let result = early.unwrap_or_else(|_| returned.recv_timeout(Duration::from_secs(3)).unwrap());
    worker.join().unwrap();
    let expired = Instant::now() + Duration::from_secs(3);
    while !native.is_complete() {
        assert!(Instant::now() < expired);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        early_return,
        mode == "normal",
        "T1421 process exit abandoned original native source"
    );
    outcome(mode, result);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(
            Following::start(factory, Default::default(), Default::default()).is_err(),
            "T1421 source admission reopened"
        );
    });
}
fn outcome(mode: &str, result: std::thread::Result<std::io::Result<Result<(), &'static str>>>) {
    match mode {
        "panic" => assert!(result.is_err()),
        "error" => assert_eq!(result.unwrap().unwrap(), Err("T1421 work error")),
        _ => assert_eq!(result.unwrap().unwrap(), Ok(())),
    }
}
