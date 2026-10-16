//! Native/async capture tasks retain admission through actual task destruction.
use anyhow::Result;
#[cfg(test)]
use blent_config::retirement::Pool;
use blent_config::retirement::{Admission, Completion, Observe};
use futures_util::FutureExt;
use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::task::{JoinError, JoinHandle};
pub struct Task {
    handle: Option<JoinHandle<Result<()>>>,
    admission: Option<Admission>,
    completion: Completion,
    leases: Vec<Box<dyn Send + Sync>>,
}
impl Task {
    fn owned(admission: Admission, handle: JoinHandle<Result<()>>) -> Self {
        Self {
            completion: admission.completion(),
            handle: Some(handle),
            admission: Some(admission),
            leases: Vec::new(),
        }
    }
    pub fn spawn(work: impl Future<Output = Result<()>> + Send + 'static) -> io::Result<Self> {
        let admission = crate::work_admission::reserve()?;
        Ok(Self::owned(admission, tokio::spawn(work)))
    }
    pub fn spawn_blocking(work: impl FnOnce() -> Result<()> + Send + 'static) -> io::Result<Self> {
        let admission = crate::work_admission::reserve()?;
        Ok(Self::owned(admission, tokio::task::spawn_blocking(work)))
    }
    pub fn abort(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
    /// The work result is ready; resource retirement is observed separately
    /// through completion(), including any post-join dependency cleanup.
    pub fn is_finished(&self) -> bool {
        match &self.handle {
            Some(handle) => handle.is_finished(),
            None => true,
        }
    }
    /// The task result has been consumed. This is not native retirement proof;
    /// original dependent cleanup can still be pending through completion().
    pub fn is_joined(&self) -> bool {
        self.handle.is_none()
    }
    pub fn completion(&self) -> Completion {
        self.completion.clone()
    }
    /// Keep original dependent ownership through actual task destruction AND join.
    pub fn keep_alive(&mut self, lease: impl Send + Sync + 'static) {
        self.leases.push(Box::new(lease));
    }
    fn retire_leases(&mut self) {
        let admission = self.admission.take().unwrap();
        if self.leases.is_empty() {
            drop(admission);
        } else {
            let leases = std::mem::take(&mut self.leases);
            admission.retain(blent_config::retirement_work::Cleanup::new(move || {
                drop(leases)
            }));
        }
    }
}
impl Future for Task {
    type Output = Result<Result<()>, JoinError>;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self.handle.as_mut().unwrap().poll_unpin(context);
        if result.is_ready() {
            self.handle.take();
            self.retire_leases();
        }
        result
    }
}
impl Drop for Task {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            self.admission.take().unwrap().retain(Retiring {
                handle: Some(handle),
                leases: std::mem::take(&mut self.leases),
                cleanup: None,
            });
        }
    }
}
struct Retiring {
    handle: Option<JoinHandle<Result<()>>>,
    leases: Vec<Box<dyn Send + Sync>>,
    cleanup: Option<blent_config::retirement_work::Cleanup>,
}
impl Retiring {
    fn joined(&mut self) -> bool {
        let Some(handle) = self.handle.as_mut() else {
            return true;
        };
        if !handle.is_finished() {
            return false;
        }
        let Some(result) = handle.now_or_never() else {
            return false;
        };
        self.handle.take();
        report_outcome(result);
        true
    }
}
impl Observe for Retiring {
    fn poll_complete(&mut self) -> bool {
        if !self.joined() {
            return false;
        }
        if let Some(cleanup) = &mut self.cleanup {
            return cleanup.poll_complete();
        }
        if self.leases.is_empty() {
            return true;
        }
        let leases = std::mem::take(&mut self.leases);
        self.cleanup = Some(blent_config::retirement_work::Cleanup::new(move || {
            drop(leases)
        }));
        self.cleanup.as_mut().unwrap().poll_complete()
    }
}

fn report_outcome(result: Result<Result<()>, JoinError>) {
    match result {
        Ok(Err(error)) => tracing::error!("Retired capture task failed: {error:#}"),
        Err(error) if !error.is_cancelled() => {
            tracing::error!("Retired capture task panicked: {error}")
        }
        _ => {}
    }
}
#[cfg(test)]
#[path = "owned_task/lease_tests.rs"]
mod lease_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[tokio::test]
    async fn t814_owned_task_keeps_native_capacity_until_real_completion() {
        let pool = Pool::new(1).unwrap();
        let admission = pool.reserve().unwrap();
        let (start, entered) = tokio::sync::oneshot::channel();
        let (release, hold) = std::sync::mpsc::channel();
        let native = Task::owned(
            admission,
            tokio::task::spawn_blocking(move || {
                start.send(()).unwrap();
                hold.recv().unwrap();
                Ok(())
            }),
        );
        let completed = native.completion();
        entered.await.unwrap();
        drop(native);
        assert!(!completed.is_complete());
        assert!(pool.reserve().is_err());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while !completed.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        assert!(pool.reserve().is_ok());
    }
    #[tokio::test]
    async fn t814_owned_task_observes_success_errors_and_panics() {
        let good = Task::spawn(async { Ok(()) }).unwrap();
        let ticket = good.completion();
        good.await.unwrap().unwrap();
        assert!(ticket.is_complete());
        assert!(Task::spawn_blocking(|| anyhow::bail!("T814 failure"))
            .unwrap()
            .await
            .unwrap()
            .is_err());
        for outcome in [0, 1, 2] {
            let task = Task::spawn(async move {
                match outcome {
                    0 => Ok(()),
                    1 => anyhow::bail!("T814 late error"),
                    _ => panic!("T814 late panic"),
                }
            })
            .unwrap();
            let ticket = task.completion();
            tokio::time::sleep(Duration::from_millis(1)).await;
            drop(task);
            tokio::time::timeout(Duration::from_secs(2), async {
                while !ticket.is_complete() {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
        }
        let mut task = Task::spawn(std::future::pending()).unwrap();
        task.abort();
        assert!((&mut task).await.unwrap_err().is_cancelled());
        task.abort();
    }
}
