//! Native storage adapter: a reusable, bounded thread pool owns blocking calls.
//! Awaiter cancellation cannot release admission around still-running native work.
use crate::{
    control_workers::{Worker, WorkerPool},
    persistence_port::*,
};
use anyhow::{Context, Result};
use blent_config::{
    model::FileConfig,
    persistence_contract::{Edit, Snapshots, Transactions},
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use tokio::sync::oneshot;

trait Backend: Snapshots + Transactions {}
impl<T: Snapshots + Transactions> Backend for T {}
enum Operation {
    Snapshot,
    Edit(Edit<'static>),
}
struct Request {
    backend: Arc<dyn Backend>,
    operation: Operation,
    cancelled: Arc<AtomicBool>,
    reply: oneshot::Sender<Result<FileConfig>>,
    _admission: Admission,
}
struct Admission(Arc<AtomicUsize>);
impl Drop for Admission {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Cancellation(Arc<AtomicBool>);
impl Drop for Cancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct QueueOwner {
    sender: Option<mpsc::SyncSender<Request>>,
    workers: Vec<Worker>,
}
struct Executor {
    queue: Mutex<QueueOwner>,
    stopped: Arc<AtomicBool>,
}
impl Executor {
    fn new(pool: WorkerPool, count: usize) -> Result<Arc<Self>> {
        anyhow::ensure!(
            (1..=64).contains(&count),
            "Configuration workers must be 1..64"
        );
        let (sender, receiver) = mpsc::sync_channel(count);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut executor = Self {
            queue: Mutex::new(QueueOwner {
                sender: Some(sender),
                workers: Vec::new(),
            }),
            stopped: Arc::new(AtomicBool::new(false)),
        };
        for _ in 0..count {
            let receiver = receiver.clone();
            let stopped = executor.stopped.clone();
            executor
                .queue
                .get_mut()
                .unwrap()
                .workers
                .push(Worker::spawn(
                    pool.clone(),
                    "blent-config-query",
                    move || run(receiver, stopped),
                )?);
        }
        Ok(Arc::new(executor))
    }
    fn submit(&self, request: Request) -> Result<()> {
        crate::process_retirement::PROCESS.check_running()?;
        let result = {
            let queue = self.queue.lock().unwrap();
            match &queue.sender {
                Some(sender) => sender.try_send(request),
                None => Err(mpsc::TrySendError::Disconnected(request)),
            }
        };
        // Rejected closures/backends are dropped after the queue lock is released.
        result.map_err(|_| anyhow::anyhow!("Configuration worker queue busy or unavailable"))
    }
    fn close(&self) {
        let workers = {
            let mut queue = self.queue.lock().unwrap();
            self.stopped.store(true, Ordering::Release);
            drop(queue.sender.take());
            std::mem::take(&mut queue.workers)
        };
        drop(workers); // Reaper retains original thread/TLS ownership, without joining here.
    }
}
impl Drop for Executor {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct Store {
    backend: Arc<dyn Backend>,
    executor: Arc<Executor>,
    admitted: Arc<AtomicUsize>,
}
impl Store {
    pub fn new(store: impl Snapshots + Transactions + 'static) -> Result<Self> {
        Ok(Self::on(store, executor()?))
    }
    /// Dedicated one-worker executor for a caller-supplied retirement budget.
    pub fn with_pool(
        store: impl Snapshots + Transactions + 'static,
        pool: WorkerPool,
    ) -> Result<Self> {
        Ok(Self::on(store, Executor::new(pool, 1)?))
    }
    fn on(store: impl Snapshots + Transactions + 'static, executor: Arc<Executor>) -> Self {
        Self {
            backend: Arc::new(store),
            executor,
            admitted: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn reserve(&self) -> Result<Admission> {
        self.admitted
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map_err(|_| anyhow::anyhow!("Configuration owner already has two pending requests"))?;
        Ok(Admission(self.admitted.clone()))
    }
    async fn request(&self, operation: Operation) -> Result<FileConfig> {
        let admission = self.reserve()?;
        let cancelled = Cancellation(Arc::new(AtomicBool::new(false)));
        let (reply, response) = oneshot::channel();
        self.executor.submit(Request {
            backend: self.backend.clone(),
            operation,
            cancelled: cancelled.0.clone(),
            reply,
            _admission: admission,
        })?;
        response.await.context("Configuration request failed")?
    }
}
impl AsyncSnapshots for Store {
    fn snapshot(&self) -> Reply<'_> {
        Box::pin(self.request(Operation::Snapshot))
    }
}
impl AsyncTransactions for Store {
    fn transact(&self, edit: Edit<'static>) -> Reply<'_> {
        Box::pin(self.request(Operation::Edit(edit)))
    }
}
fn run(receiver: Arc<Mutex<mpsc::Receiver<Request>>>, stopped: Arc<AtomicBool>) {
    loop {
        let received = { receiver.lock().unwrap().recv() };
        let Ok(request) = received else { return };
        // No user/storage code runs with the queue lock. Discarding a queued
        // closure/backend is also user code and retains the same panic boundary.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if stopped.load(Ordering::Acquire) {
                drop(request);
            } else {
                execute(request);
            }
        }));
    }
}
fn execute(request: Request) {
    if request.reply.is_closed() || request.cancelled.load(Ordering::Acquire) {
        return;
    }
    let result = match request.operation {
        Operation::Snapshot => Ok(request.backend.snapshot()),
        Operation::Edit(edit) => request.backend.transact(&request.cancelled, edit),
    };
    let _ = request.reply.send(result);
}
#[derive(Default)]
struct Shared {
    closed: bool,
    executor: Option<Result<Arc<Executor>, String>>,
}
impl Shared {
    fn get(&mut self) -> Result<Arc<Executor>> {
        if self.closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                crate::runtime_owner::AdmissionClosed,
            )
            .into());
        }
        self.executor
            .get_or_insert_with(|| {
                configured_pool(std::env::var("BLENT_CONFIG_WORKERS"))
                    .map_err(|error| error.to_string())
            })
            .as_ref()
            .cloned()
            .map_err(|error| anyhow::anyhow!(error.clone()))
    }
    fn close(&mut self) {
        self.closed = true;
        if let Some(Ok(executor)) = &self.executor {
            executor.close();
        }
    }
}
// The gate serializes original construction with process cutoff. A late
// initializer cannot publish a static worker after shutdown inspected the slot.
static SHARED: Mutex<Shared> = Mutex::new(Shared {
    closed: false,
    executor: None,
});
fn executor() -> Result<Arc<Executor>> {
    SHARED.lock().unwrap().get()
}
pub(crate) fn shutdown() {
    SHARED.lock().unwrap().close();
}
fn configured_pool(value: Result<String, std::env::VarError>) -> Result<Arc<Executor>> {
    let capacity = match value {
        Ok(text) => text.parse().context("Invalid BLENT_CONFIG_WORKERS")?,
        Err(std::env::VarError::NotPresent) => 2,
        Err(error) => return Err(error.into()),
    };
    Executor::new(WorkerPool::new(capacity)?, capacity)
}

#[cfg(test)]
mod tests;
