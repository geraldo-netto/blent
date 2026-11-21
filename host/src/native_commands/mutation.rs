//! A funded port retains one original mutation result independently of its waiter.
use super::*;
use futures_util::FutureExt;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub(super) struct Report {
    pub settled: bool,
    pub result: io::Result<()>,
}
impl Report {
    pub(super) fn rejected() -> Self {
        Self {
            settled: true,
            result: Err(io::Error::other("native mutation rejected before dispatch")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Pending,
    Settled,
    Unknown,
}

pub(super) struct Operation {
    worker: Option<tokio::task::JoinHandle<()>>,
    outcome: Arc<Mutex<Outcome>>,
}
impl Operation {
    #[cfg(test)]
    pub(super) fn start(
        program: &str,
        args: &[&str],
        receipts: [Reservation; 2],
    ) -> (Self, tokio::sync::oneshot::Receiver<io::Result<Output>>) {
        let program = program.to_owned();
        let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        let outcome = Arc::new(Mutex::new(Outcome::Pending));
        let completion = outcome.clone();
        let (send, result) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            let result = execute(&program, &args, receipts, &completion).await;
            let _ = send.send(result);
        });
        (
            Self {
                worker: Some(worker),
                outcome,
            },
            result,
        )
    }
    pub(super) fn acknowledged(
        work: impl Future<Output = Report> + Send + 'static,
    ) -> (Self, tokio::sync::oneshot::Receiver<io::Result<()>>) {
        let outcome = Arc::new(Mutex::new(Outcome::Pending));
        let completion = outcome.clone();
        let (send, result) = tokio::sync::oneshot::channel();
        let worker = tokio::spawn(async move {
            let report = work.await;
            *completion.lock().unwrap() = if report.settled {
                Outcome::Settled
            } else {
                Outcome::Unknown
            };
            let _ = send.send(report.result);
        });
        (
            Self {
                worker: Some(worker),
                outcome,
            },
            result,
        )
    }
    pub(super) fn complete(&mut self) -> bool {
        if let Some(worker) = self.worker.as_mut() {
            if !worker.is_finished() {
                return false;
            }
            let Some(result) = worker.now_or_never() else {
                return false;
            };
            self.worker = None;
            if result.is_err() {
                *self.outcome.lock().unwrap() = Outcome::Unknown;
            }
        }
        *self.outcome.lock().unwrap() == Outcome::Settled
    }
}
#[cfg(test)]
async fn execute(
    program: &str,
    args: &[String],
    receipts: [Reservation; 2],
    outcome: &Mutex<Outcome>,
) -> io::Result<Output> {
    let receipts = Mutex::new(receipts.map(Some));
    let reported = AtomicUsize::new(0);
    let observe = |proof| {
        receipts
            .lock()
            .unwrap()
            .iter_mut()
            .find_map(Option::take)
            .expect("command reports at most collector and original child")
            .bind(proof);
        reported.fetch_add(1, Ordering::AcqRel);
    };
    let result = tokio::process::Command::new(program)
        .args(args)
        .output_input_retained_observed(None, &observe)
        .await;
    let observed = classify(&result, reported.load(Ordering::Acquire));
    if observed == Outcome::Unknown {
        tracing::warn!(program, status = ?result.as_ref().map(|output| output.status),
            "Native mutation outcome unconfirmed; original resources remain quarantined");
    }
    *outcome.lock().unwrap() = observed;
    result
}
#[cfg(test)]
fn classify(result: &io::Result<Output>, reported: usize) -> Outcome {
    if reported < 2 {
        return Outcome::Settled;
    }
    if reported == 2 && result.as_ref().is_ok_and(|output| output.status.success()) {
        return Outcome::Settled;
    }
    // Generic CLI failure cannot distinguish a remote rejection from a lost
    // reply/transport or partial effect. Never turn process exit into that proof.
    Outcome::Unknown
}

#[cfg(test)]
mod tests;
