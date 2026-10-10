//! Owned native children. Capacity includes active and unconfirmed retirement.
use super::{prepare_group, retirement, terminate_group, timed_out};
use crate::retirement::{Admission, Completion};
use std::{
    io,
    ops::{Deref, DerefMut},
    process::ExitStatus,
    time::Duration,
};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetirementPhase {
    Exited,
    ReapedAfterSignal,
    Incomplete,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetirementError {
    /// Grace plus reap cannot be represented by the monotonic clock.
    InvalidGrace,
    Signal(String),
}
#[derive(Clone, Debug)]
pub struct Retirement {
    pub phase: RetirementPhase,
    pub completion: Completion,
    pub error: Option<RetirementError>,
}
pub struct OwnedChild {
    child: Option<Child>,
    admission: Option<Admission>,
    reap_budget: Duration,
    leases: Vec<Box<dyn Send + Sync>>,
    #[cfg(windows)]
    job: Option<super::windows::Job>,
}
impl OwnedChild {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        let admission = retirement::reserve()?;
        let reap_budget = crate::retirement_limits::reap_budget()?;
        #[cfg(windows)]
        let job = super::windows::Job::new()?;
        prepare_group(command.as_std_mut());
        let child = command.kill_on_drop(true).spawn()?;
        let owner = Self {
            child: Some(child),
            admission: Some(admission),
            reap_budget,
            leases: Vec::new(),
            #[cfg(windows)]
            job: Some(job),
        };
        #[cfg(windows)]
        owner.job.as_ref().unwrap().start(
            owner.id().expect("new child"),
            owner.raw_handle().expect("new child handle"),
        )?;
        Ok(owner)
    }
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().unwrap().id()
    }
    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.stdin.take()
    }
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }
    /// Keep dependent locks/buffers alive until the direct child is reaped.
    pub fn keep_alive(&mut self, lease: impl Send + Sync + 'static) {
        self.leases.push(Box::new(lease));
    }
    pub fn completion(&self) -> Completion {
        self.admission.as_ref().unwrap().completion()
    }
    /// Consume the child. On expiry, signal and transfer ownership to the reaper;
    /// completion refers to the direct child, never delegated service work.
    pub async fn finish(mut self, timeout: Duration) -> io::Result<ExitStatus> {
        tokio::time::timeout(timeout, self.wait())
            .await
            .map_err(|_| timed_out())?
    }
    fn signal(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        drop(self.job.take()); // kill-on-close covers the Windows job tree
        if let Some(pid) = self.id() {
            terminate_group(pid);
        }
        self.start_kill()
    }
    /// Grace, signal, bounded reap, then retain any unconfirmed native owner.
    /// Invalid grace returns an explicit error; Drop still signals and retains
    /// the child and its leases without waiting for native completion.
    pub async fn retire(mut self, grace: Duration) -> Retirement {
        let completion = self.completion();
        let Some(deadline) = tokio::time::Instant::now()
            .checked_add(grace)
            .and_then(|end| end.checked_add(self.reap_budget))
        else {
            return Retirement {
                phase: RetirementPhase::Incomplete,
                completion,
                error: Some(RetirementError::InvalidGrace),
            };
        };
        drop(self.stdin.take());
        let mut error = None;
        let mut phase = if matches!(tokio::time::timeout(grace, self.wait()).await, Ok(Ok(_))) {
            RetirementPhase::Exited
        } else {
            error = self
                .signal()
                .err()
                .map(|error| RetirementError::Signal(error.to_string()));
            let budget = self.reap_budget;
            if matches!(tokio::time::timeout(budget, self.wait()).await, Ok(Ok(_))) {
                RetirementPhase::ReapedAfterSignal
            } else {
                RetirementPhase::Incomplete
            }
        };
        drop(self);
        let _ = tokio::time::timeout_at(deadline, async {
            while !completion.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await;
        if !completion.is_complete() {
            phase = RetirementPhase::Incomplete;
        }
        Retirement {
            phase,
            completion,
            error,
        }
    }
}
impl Deref for OwnedChild {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        self.child.as_ref().unwrap()
    }
}
impl DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.child.as_mut().unwrap()
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.signal();
        let mut child = self.child.take().unwrap();
        let admission = self.admission.take().unwrap();
        drop(child.stdin.take());
        drop(child.stdout.take());
        drop(child.stderr.take());
        if child.id().is_some() || !self.leases.is_empty() {
            admission.retain(retirement::AsyncChild {
                child,
                leases: std::mem::take(&mut self.leases),
                cleanup: None,
            });
        }
        // Reaped children release their admission here; unreaped ones retain it.
    }
}
