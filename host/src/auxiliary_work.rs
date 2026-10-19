//! Original auxiliary work, independent of OS observation and native adapters.
//! `start` owns one attempt and observes its one-way stop receiver. `settle`
//! must wait for that attempt's actual native obligations, including failures;
//! logical completion is not its fence. Both callbacks remain borrowed for this
//! workflow. Each caller owns its independent manual intent and resource budget.
//! Dropping this workflow closes the attempt's stop channel. Adapters must retain
//! uncertain native work independently when their future is dropped; the caller
//! keeps polling through settlement for graceful stop and automatic resume.
use crate::activity_view::View;
use anyhow::Result;
use blent_config::host_activity::{ActivityTarget, Publication, WorkObservation};
use std::future::Future;
use tokio::sync::watch;

#[derive(Clone, Copy)]
enum End {
    Finished,
    Paused,
}

pub async fn run<F, Fut, S, Settled>(
    source: Option<watch::Receiver<Publication>>,
    mut stop: watch::Receiver<bool>,
    start: F,
    settle: S,
) -> Result<()>
where
    F: Fn(watch::Receiver<bool>) -> Fut,
    Fut: Future<Output = Result<()>>,
    S: Fn() -> Settled,
    Settled: Future<Output = ()>,
{
    let activity = View::new(ActivityTarget::Auxiliary, source);
    while let Some(original) = admit(&activity, &mut stop).await {
        match attempt(&activity, original, &mut stop, &start, &settle).await? {
            End::Finished => return Ok(()),
            End::Paused => (),
        }
    }
    Ok(())
}

async fn admit(activity: &View, stop: &mut watch::Receiver<bool>) -> Option<WorkObservation> {
    loop {
        let change = activity.changed();
        if *stop.borrow_and_update() || stop.has_changed().is_err() {
            return None;
        }
        if let Some(original) = activity.begin() {
            return Some(original);
        }
        tokio::select! { biased;
            _ = stopped(stop) => return None,
            _ = change => (),
        }
    }
}

async fn withdrawn(activity: &View, original: WorkObservation) {
    loop {
        let change = activity.changed();
        if !activity.permits(original) {
            return;
        }
        change.await;
    }
}

async fn stopped(stop: &mut watch::Receiver<bool>) {
    loop {
        if *stop.borrow_and_update() {
            return;
        }
        if stop.changed().await.is_err() {
            return;
        }
    }
}

async fn attempt<F, Fut, S, Settled>(
    activity: &View,
    original: WorkObservation,
    manual: &mut watch::Receiver<bool>,
    start: &F,
    settle: &S,
) -> Result<End>
where
    F: Fn(watch::Receiver<bool>) -> Fut,
    Fut: Future<Output = Result<()>>,
    S: Fn() -> Settled,
    Settled: Future<Output = ()>,
{
    let (stop, stopped_attempt) = watch::channel(false);
    let work = crate::control_outcome::run(|| start(stopped_attempt));
    tokio::pin!(work);
    let end = tokio::select! { biased;
        _ = stopped(manual) => End::Finished,
        _ = withdrawn(activity, original) => End::Paused,
        result = &mut work => {
            settle().await;
            return result.map(|()| End::Finished);
        }
    };
    stop.send_replace(true);
    let result = work.await;
    settle().await;
    result?;
    Ok(end)
}

#[cfg(test)]
mod tests;
