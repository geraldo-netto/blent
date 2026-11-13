//! Review-only original native activity ownership; no OS activity/device access.
use blent::activity_follow::Following;
use blent_config::{
    host_activity::*,
    retirement::{Completion, Pool},
};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
struct Factory {
    release: Mutex<Option<mpsc::Receiver<()>>>,
    native: Mutex<Option<Completion>>,
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
        Box::pin(async move {
            Ok(StartedSource {
                epoch: 1,
                source: Box::new(source),
            })
        })
    }
}
async fn work(factory: Arc<Factory>, settled: bool, release: mpsc::Sender<()>) {
    let source = Following::start(factory.clone(), Default::default(), Default::default()).unwrap();
    while factory.native.lock().unwrap().is_none() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    source.stop();
    if settled {
        release.send(()).unwrap();
    }
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        while !source.retired() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert_eq!(result.is_ok(), settled);
    drop(source);
}
fn main() {
    let settled = std::env::var_os("BLENT_REVIEW_SETTLED").is_some();
    let (release, hold) = mpsc::channel();
    let factory = Arc::new(Factory {
        release: Mutex::new(Some(hold)),
        native: Mutex::new(None),
    });
    let started = Instant::now();
    blent::runtime_owner::run(work(factory.clone(), settled, release.clone())).unwrap();
    let native = factory.native.lock().unwrap().clone().unwrap();
    println!(
        "runtime_returned=true native_complete={} settled_control={} elapsed_ms={}",
        native.is_complete(),
        settled,
        started.elapsed().as_millis()
    );
    assert_eq!(native.is_complete(), settled);
    if !settled {
        release.send(()).unwrap();
    }
    while !native.is_complete() {
        std::thread::sleep(Duration::from_millis(1));
    }
    println!("owned_native_thread_released_and_joined=true");
}
