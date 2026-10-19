//! The source runtime outlives individual client runtimes and retains real joins.
use super::*;
use blent_config::{
    retirement::{Admission, Observe},
    retirement_work::{Cleanup, Thread},
};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use tokio::runtime::Runtime;

type Reply = SyncSender<Result<Arc<Following>>>;
type Response = Receiver<Result<Arc<Following>>>;
pub(super) struct Worker {
    owned: Option<(Admission, Thread)>,
    source: Option<Arc<Following>>,
    stop: watch::Sender<bool>,
    interval: std::time::Duration,
}
impl Worker {
    pub(super) fn attach(&mut self, source: Arc<Following>) {
        assert!(self.source.is_none());
        self.source = Some(source);
    }
    pub(super) fn updates(&self) -> watch::Receiver<Publication> {
        self.source
            .as_ref()
            .expect("attached original source")
            .subscribe()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        // This private channel lends no read guards to consumers. Do not call
        // Following.stop() here: a consumer may still hold its publication view.
        self.stop.send_replace(true);
        if let Some((admission, thread)) = self.owned.take() {
            admission.retain(Retiring {
                thread,
                source: self.source.take(),
                cleanup: None,
                interval: self.interval,
            });
        }
    }
}
struct Retiring {
    thread: Thread,
    source: Option<Arc<Following>>,
    cleanup: Option<Cleanup>,
    interval: std::time::Duration,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        if !self.thread.poll_complete() {
            return false;
        }
        if let Some(cleanup) = &mut self.cleanup {
            return cleanup.poll_complete();
        }
        let Some(source) = self.source.take() else {
            return true;
        };
        let interval = self.interval;
        self.cleanup = Some(Cleanup::new(move || settle_source(source, interval)));
        self.cleanup.as_mut().unwrap().poll_complete()
    }
}

fn settle_source(source: Arc<Following>, interval: std::time::Duration) {
    source.stop();
    // Worker unwind may leave a joined task's independent backend obligations.
    // Poll off the shared reaper and retain the original admission throughout.
    while !source.retired() {
        std::thread::sleep(interval);
    }
    drop(source);
}

pub(super) fn start(
    pool: &Pool,
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
) -> Result<(Worker, Response)> {
    let admission = pool.reserve()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (reply, response) = sync_channel(1);
    let (stop, stopped) = watch::channel(false);
    let thread = std::thread::Builder::new()
        .name("blent-aux-activity".into())
        .spawn(move || work(runtime, factory, settings, stopped, reply))?;
    Ok((
        Worker {
            owned: Some((admission, Thread::new(thread))),
            source: None,
            stop,
            interval: settings.interval,
        },
        response,
    ))
}

pub(super) fn work(
    runtime: Runtime,
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    stop: watch::Receiver<bool>,
    reply: Reply,
) {
    let errors = reply.clone();
    let panics = reply.clone();
    crate::control_outcome::worker(
        || {
            if let Err(error) = runtime.block_on(follow(factory, settings, stop, reply)) {
                let _ = errors.try_send(Err(error));
            }
        },
        |error| {
            let _ = panics.try_send(Err(error));
        },
    );
}

struct Stop(Arc<Following>);
impl Drop for Stop {
    fn drop(&mut self) {
        self.0.stop();
    }
}
async fn follow(
    factory: Arc<dyn SourceFactory>,
    settings: Settings,
    mut stop: watch::Receiver<bool>,
    reply: Reply,
) -> Result<()> {
    let source = Arc::new(Following::start(factory, settings, Default::default())?);
    // Guard before exposure: an unwinding worker must close its original view
    // before runtime teardown, even while another frontend retains the source.
    let _stop = Stop(source.clone());
    if reply.try_send(Ok(source.clone())).is_err() {
        source.stop();
    }
    let _ = stop.wait_for(|value| *value).await;
    source.stop();
    while !source.retired() {
        tokio::time::sleep(settings.interval).await;
    }
    Ok(())
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
