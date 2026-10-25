use super::{native, Control, Invocation};
use anyhow::{ensure, Result};
use blent_config::adb_reverse::temporary::{self, Cleanup, Creation, Route};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

pub(super) async fn run(
    control: Arc<Control>,
    mut requests: mpsc::Receiver<Invocation>,
    mut stop: watch::Receiver<bool>,
    ready: oneshot::Sender<Result<(), String>>,
    failure: watch::Sender<Option<String>>,
) {
    let mut creation = Creation::NotStarted;
    let created = create(&control, &mut creation).await;
    let active = created.is_ok();
    let _ = ready.send(created.map_err(|error| error.to_string()));
    if active {
        serve(&control, &mut requests, &mut stop).await;
    }
    requests.close();
    cleanup(&control, creation, &failure).await;
    drop(control.endpoints.lock().unwrap().take());
    control.settled.store(true, Ordering::Release);
}

async fn create(control: &Control, creation: &mut Creation) -> Result<()> {
    let target = &control.target;
    let listing = native::list(control).await?;
    ensure!(
        temporary::route(&listing, &target.remote, &target.local)? == Route::Missing,
        "ADB temporary endpoint already exists"
    );
    *creation = Creation::Uncertain;
    let attempt = native::execute(
        control,
        &[
            "-s",
            &target.serial,
            "reverse",
            "--no-rebind",
            &target.remote,
            &target.local,
        ],
        None,
    )
    .await;
    if !attempt.started {
        *creation = Creation::NotStarted;
    }
    ensure!(
        attempt.output?.status.success(),
        "ADB temporary mapping creation failed"
    );
    *creation = Creation::Acknowledged;
    Ok(())
}

async fn serve(
    control: &Control,
    requests: &mut mpsc::Receiver<Invocation>,
    stop: &mut watch::Receiver<bool>,
) {
    while !*stop.borrow() {
        tokio::select! {
            biased;
            _ = stop.changed() => break,
            request = requests.recv() => {
                let Some(request) = request else { break };
                let result = native::shell(control, &request.input).await;
                let _ = request.result.send(result.map_err(|error| error.to_string()));
            }
        }
    }
}

async fn cleanup(
    control: &Control,
    mut creation: Creation,
    failure: &watch::Sender<Option<String>>,
) {
    loop {
        match cleanup_once(control, &mut creation).await {
            Ok(true) => return,
            Ok(false) => {
                if creation == Creation::Uncertain {
                    failure.send_replace(Some("ADB creation outcome remains unknown".into()));
                }
            }
            Err(error) => {
                failure.send_replace(Some(error.to_string()));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn cleanup_once(control: &Control, creation: &mut Creation) -> Result<bool> {
    if *creation == Creation::NotStarted {
        return Ok(true);
    }
    let target = &control.target;
    let listing = native::list(control).await?;
    let route = temporary::route(&listing, &target.remote, &target.local)?;
    match creation.cleanup(route) {
        Cleanup::Complete => Ok(true),
        Cleanup::Wait => Ok(false),
        Cleanup::Remove => {
            *creation = Creation::RemovalPending;
            let attempt = native::execute(
                control,
                &["-s", &target.serial, "reverse", "--remove", &target.remote],
                None,
            )
            .await;
            if !attempt.started {
                *creation = Creation::Acknowledged;
            }
            ensure!(
                attempt.output?.status.success(),
                "ADB mapping removal failed"
            );
            *creation = Creation::Acknowledged;
            Ok(false)
        }
    }
}
