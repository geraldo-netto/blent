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
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::watch,
    task::{JoinError, JoinHandle},
};

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
pub struct Workers {
    capture: JoinHandle<Result<()>>,
    other: FuturesUnordered<NamedTask>,
}
impl Workers {
    pub fn new(
        capture: JoinHandle<Result<()>>,
        tasks: Vec<(&'static str, JoinHandle<Result<()>>)>,
    ) -> Self {
        Self {
            capture,
            other: tasks
                .into_iter()
                .map(|(name, handle)| NamedTask { name, handle })
                .collect(),
        }
    }
    async fn first(&mut self, stop: &mut watch::Receiver<bool>) -> Wake {
        tokio::select! {
            biased;
            _ = stop.wait_for(|stop|*stop) => Wake::Stop,
            result = &mut self.capture => Wake::Capture(result),
            Some((name,result)) = self.other.next(), if !self.other.is_empty() => Wake::Other(name,result),
        }
    }
    async fn retire(&mut self, capture_done: bool) {
        for task in &self.other {
            task.handle.abort();
        }
        if !capture_done
            && tokio::time::timeout(Duration::from_secs(5), &mut self.capture)
                .await
                .is_err()
        {
            tracing::warn!("Capture did not stop within 5s");
            self.capture.abort();
            let _ = (&mut self.capture).await;
        }
        while self.other.next().await.is_some() {}
    }
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.capture.abort();
        for task in &self.other {
            task.handle.abort();
        }
    }
}
enum Wake {
    Stop,
    Capture(Completion),
    Other(&'static str, Completion),
}
impl Wake {
    fn failure(self) -> Option<Failure> {
        let (worker, result) = match self {
            Self::Stop => return None,
            Self::Capture(result) => ("Capture", result),
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
    let mut stop = stopped.subscribe();
    let wake = workers.first(&mut stop).await;
    let capture_done = matches!(wake, Wake::Capture(_));
    let failure = if *stop.borrow() { None } else { wake.failure() };
    tablet.retire();
    stopped.send_replace(true);
    if let Some(failure) = &failure {
        tracing::error!("{} worker failed: {}", failure.worker, failure.message);
        status.send_replace(State::Failed(failure.clone()));
    }
    workers.retire(capture_done).await;
    if failure.is_none() {
        status.send_replace(State::Stopped);
    }
}
