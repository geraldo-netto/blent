//! One owner observes terminal outcomes and retains all session task handles.
use crate::{
    attachment::Attachment,
    session_state::{Failure, State},
};
use anyhow::Result;
use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use std::{
    future::Future,
    pin::Pin,
    sync::OnceLock,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::watch,
    task::{JoinError, JoinHandle},
};

use blent_config::retirement::{Admission, Observe, Pool};

type Completion = Result<Result<()>, JoinError>;
struct NamedTask {
    name: &'static str,
    handle: JoinHandle<Result<()>>,
}
impl Future for NamedTask {
    type Output = (&'static str, Completion);
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.handle
            .poll_unpin(context)
            .map(|result| (self.name, result))
    }
}
impl Drop for NamedTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
/// Admission is acquired before the caller starts any session worker.
pub struct SessionAdmission {
    slot: Admission,
    grace: Duration,
    reap: Duration,
    creation: Option<std::sync::Arc<dyn crate::capture_contract::NativeRetirement>>,
    native: crate::capture_retirement::Observation,
}
impl SessionAdmission {
    /// Independent observer retained before startup is exposed to a task.
    pub fn completion(&self) -> blent_config::retirement::Completion {
        self.slot.completion()
    }
}
pub struct Workers {
    tasks: FuturesUnordered<NamedTask>,
    admission: Option<SessionAdmission>,
    status: Option<watch::Sender<State>>,
}
impl Workers {
    pub fn reserve() -> std::io::Result<SessionAdmission> {
        static POOL: OnceLock<std::io::Result<Pool>> = OnceLock::new();
        let pool = POOL.get_or_init(|| {
            Pool::new(blent_config::retirement_limits::configured(
                "BLENT_SESSION_LIMIT",
                64,
                4096,
            )?)
        });
        let slot = pool
            .as_ref()
            .map_err(|error| std::io::Error::new(error.kind(), error.to_string()))?
            .reserve()?;
        let grace = Duration::from_millis(blent_config::retirement_limits::configured(
            "BLENT_SESSION_STOP_MS",
            5000,
            60_000,
        )? as u64);
        let reap = blent_config::retirement_limits::reap_budget()?;
        let (creation, native) = crate::capture_retirement::channel();
        Ok(SessionAdmission {
            slot,
            grace,
            reap,
            creation: Some(creation),
            native,
        })
    }
    pub fn new(
        admission: SessionAdmission,
        capture: JoinHandle<Result<()>>,
        mut tasks: Vec<(&'static str, JoinHandle<Result<()>>)>,
    ) -> Self {
        let mut workers = Self::empty(admission);
        tasks.push(("Capture", capture));
        for (name, handle) in tasks {
            workers.add(name, handle);
        }
        workers
    }
    /// Install retained ownership before exposing the first worker.
    pub fn empty(admission: SessionAdmission) -> Self {
        Self {
            tasks: FuturesUnordered::new(),
            admission: Some(admission),
            status: None,
        }
    }
    pub fn add(&mut self, name: &'static str, handle: JoinHandle<Result<()>>) {
        self.tasks.push(NamedTask { name, handle });
    }
    pub(crate) fn capture_lifetime(
        &mut self,
    ) -> std::sync::Arc<dyn crate::capture_contract::NativeRetirement> {
        self.admission
            .as_mut()
            .unwrap()
            .creation
            .take()
            .expect("unstarted capture lifetime")
    }
    pub(super) fn completion(&self) -> blent_config::retirement::Completion {
        self.admission.as_ref().unwrap().slot.completion()
    }
    async fn first(&mut self, stop: &mut watch::Receiver<bool>) -> Wake {
        tokio::select! {
            biased;
            _ = stop.wait_for(|stop|*stop) => Wake::Stop,
            Some((name,result)) = self.tasks.next() => Wake::Other(name,result),
        }
    }
    fn abort(&self, capture: bool) {
        for task in &self.tasks {
            if capture || task.name != "Capture" {
                task.handle.abort();
            }
        }
    }
    async fn drain(&mut self) {
        while let Some((name, result)) = self.tasks.next().await {
            publish_cleanup(self.status.as_ref(), name, result);
        }
    }
    async fn retire(&mut self) {
        drop(self.admission.as_mut().unwrap().creation.take());
        self.abort(false);
        let admission = self.admission.as_ref().unwrap();
        let (grace, reap) = (admission.grace, admission.reap);
        if tokio::time::timeout(grace, self.drain()).await.is_err() {
            self.abort(true);
            let _ = tokio::time::timeout(reap, self.drain()).await;
        }
        let complete = self.tasks.is_empty() && self.admission.as_ref().unwrap().native.complete();
        publish_retirement(self.status.as_ref(), complete);
    }
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.abort(true);
        let mut admission = self.admission.take().unwrap();
        drop(admission.creation.take());
        if !self.tasks.is_empty() || !admission.native.complete() {
            admission.slot.retain(Retiring {
                tasks: std::mem::take(&mut self.tasks),
                status: self.status.take(),
                native: admission.native,
            });
        }
    }
}
struct Retiring {
    tasks: FuturesUnordered<NamedTask>,
    status: Option<watch::Sender<State>>,
    native: crate::capture_retirement::Observation,
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        let waker = futures_util::task::noop_waker();
        let mut context = Context::from_waker(&waker);
        while let Poll::Ready(Some((name, result))) = self.tasks.poll_next_unpin(&mut context) {
            publish_cleanup(self.status.as_ref(), name, result);
        }
        if !self.tasks.is_empty() || !self.native.complete() {
            return false;
        }
        publish_retirement(self.status.as_ref(), true);
        true
    }
}
fn publish_cleanup(status: Option<&watch::Sender<State>>, name: &'static str, result: Completion) {
    let message = match result {
        Ok(Ok(())) => return,
        Ok(Err(error)) => format!("{error:#}"),
        Err(error) if error.is_cancelled() => return,
        Err(error) => error.to_string(),
    };
    tracing::error!("{name} cleanup failed: {message}");
    if let Some(status) = status {
        status.send_if_modified(|state| {
            if matches!(state, State::Failed(_)) {
                return false;
            }
            *state = State::Failed(Failure {
                worker: name,
                message,
            });
            true
        });
    }
}
fn publish_retirement(status: Option<&watch::Sender<State>>, complete: bool) {
    if let Some(status) = status {
        status.send_if_modified(|state| {
            if matches!(state, State::Failed(_) | State::Cancelled) {
                return false;
            }
            *state = if complete {
                State::Stopped
            } else {
                State::Retiring
            };
            true
        });
    }
}

enum Wake {
    Stop,
    Other(&'static str, Completion),
}
impl Wake {
    fn failure(self) -> Option<Failure> {
        let (worker, result) = match self {
            Self::Stop => return None,
            Self::Other(worker, result) => (worker, result),
        };
        let message = match result {
            Ok(Ok(())) => "Worker completed unexpectedly".into(),
            Ok(Err(error)) => format!("{error:#}"),
            Err(error) => error.to_string(),
        };
        Some(Failure { worker, message })
    }
}
pub(super) async fn run(
    mut workers: Workers,
    tablet: Attachment,
    stopped: watch::Sender<bool>,
    status: watch::Sender<State>,
) {
    workers.status = Some(status.clone());
    let mut stop = stopped.subscribe();
    let wake = workers.first(&mut stop).await;
    let failure = if *stop.borrow() { None } else { wake.failure() };
    tablet.retire();
    stopped.send_replace(true);
    if let Some(failure) = &failure {
        tracing::error!("{} worker failed: {}", failure.worker, failure.message);
        status.send_replace(State::Failed(failure.clone()));
    }
    workers.retire().await;
}

#[cfg(test)]
#[path = "session_supervision/native_tests.rs"]
mod native_tests;

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn admission(pool: &Pool) -> SessionAdmission {
        let (creation, native) = crate::capture_retirement::channel();
        SessionAdmission {
            slot: pool.reserve().unwrap(),
            grace: Duration::from_millis(10),
            reap: Duration::from_millis(10),
            creation: Some(creation),
            native,
        }
    }
    #[tokio::test]
    async fn t928_terminal_startup_worker_outcomes_remain_failures() {
        assert!(Wake::Stop.failure().is_none());
        for result in [Ok(Ok(())), Ok(Err(anyhow::anyhow!("T928 worker error")))] {
            let failure = Wake::Other("Capture", result).failure().unwrap();
            assert_eq!(failure.worker, "Capture");
            assert!(!failure.message.is_empty());
        }
        let panicked: Completion = tokio::spawn(async {
            panic!("T928 worker panic");
        })
        .await;
        assert!(panicked.as_ref().unwrap_err().is_panic());
        let failure = Wake::Other("Capture", panicked).failure().unwrap();
        assert_eq!(failure.worker, "Capture");
        assert!(failure.message.contains("T928 worker panic"));
        let cancelled = tokio::spawn(std::future::pending::<Result<()>>());
        cancelled.abort();
        let failure = Wake::Other("Capture", cancelled.await).failure().unwrap();
        assert!(failure.message.contains("cancelled"));
    }
    #[tokio::test]
    async fn t814_cleanup_error_and_panic_publish_failure() {
        for panic in [false, true] {
            let pool = Pool::new(1).unwrap();
            let admission = admission(&pool);
            let capture = tokio::spawn(async move {
                assert!(!panic, "T814 cleanup panic");
                anyhow::bail!("T814 cleanup error")
            });
            let mut workers = Workers::new(admission, capture, vec![]);
            let (status, observed) = watch::channel(State::Running);
            workers.status = Some(status);
            workers.retire().await;
            assert!(
                matches!(&*observed.borrow(),State::Failed(failure) if failure.worker=="Capture"),
                "T814: cleanup failure reported as stopped"
            );
        }
    }
    #[tokio::test]
    async fn t814_retiring_session_keeps_slot_until_native_cleanup_and_siblings_progress() {
        let pool = Pool::new(2).unwrap();
        let first = admission(&pool);
        let (started, entered) = tokio::sync::oneshot::channel();
        let (release, hold) = std::sync::mpsc::channel();
        let capture = tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            hold.recv().unwrap();
            Ok(())
        });
        entered.await.unwrap();
        let mut workers = Workers::new(first, capture, vec![]);
        let ticket = workers.completion();
        let (status, observed) = watch::channel(State::Running);
        workers.status = Some(status);
        workers.retire().await;
        assert_eq!(*observed.borrow(), State::Retiring);
        drop(workers);
        assert!(!ticket.is_complete());
        let second = admission(&pool);
        assert!(pool.reserve().is_err());
        let mut sibling = Workers::new(second, tokio::spawn(async { Ok(()) }), vec![]);
        sibling.retire().await;
        drop(sibling);
        assert_eq!(pool.active(), 1);
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !ticket.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(pool.active(), 0);
        assert_eq!(*observed.borrow(), State::Stopped);
    }
}
