use super::*;
use std::time::Duration;

pub(crate) fn failed_runtime(instance: u32) -> blent::session::Runtime {
    let admission = blent::session::Workers::reserve().unwrap();
    let capture = tokio::spawn(async { anyhow::bail!("T1510 injected worker failure") });
    blent::session::Runtime::from_workers(
        instance,
        crate::discovery_tests::monitor_inputs(2, (18000, 18001)).0,
        Default::default(),
        watch::channel(false).0,
        blent::session::Workers::new(admission, capture, vec![]),
        (0, 0),
    )
}

#[tokio::test]
async fn t1510_primary_worker_failure_ends_daemon_with_error() {
    let runtime = failed_runtime(0);
    let (_report, monitor) = watch::channel(None);
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        wait(runtime.status.clone(), monitor, std::future::pending()),
    )
    .await;
    let retired = runtime.stop().await;
    assert!(retired.is_complete());
    assert!(
        matches!(&result, Ok(Err(error)) if error.to_string().contains("T1510 injected")),
        "T1510 daemon did not observe failed primary: {result:?}"
    );
}

#[tokio::test]
async fn t1510_monitor_failure_and_channel_loss_are_fatal() {
    for close in [false, true] {
        let (_primary, status) = watch::channel(State::Running);
        let (report, monitor) = watch::channel(None);
        let task = tokio::spawn(wait(status, monitor, std::future::pending()));
        tokio::task::yield_now().await;
        if close {
            drop(report);
        } else {
            report.send_replace(Some(Failure {
                worker: "ADB display",
                message: "T1510 failure".into(),
            }));
        }
        let result = task.await.unwrap();
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn t1510_normal_shutdown_has_priority_over_simultaneous_stop_states() {
    for state in [State::Stopped, State::Retiring, State::Cancelled] {
        let (_primary, status) = watch::channel(state.clone());
        let (report, monitor) = watch::channel(None);
        drop(report);
        assert!(wait(status, monitor, async { Ok(()) }).await.is_ok());
        assert!(session_failure(&state).is_some());
    }
    let (primary, status) = watch::channel(State::Running);
    let (_report, monitor) = watch::channel(None);
    drop(primary);
    assert!(wait(status, monitor, std::future::pending())
        .await
        .unwrap_err()
        .to_string()
        .contains("Primary session status closed"));
}

#[tokio::test]
async fn t1510_delayed_primary_failure_and_shutdown_errors_are_preserved() {
    let (primary, status) = watch::channel(State::Running);
    let (_report, monitor) = watch::channel(None);
    let task = tokio::spawn(wait(status, monitor, std::future::pending()));
    tokio::task::yield_now().await;
    primary.send_replace(State::Stopped);
    assert!(task
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("Stopped"));
    let (_primary, status) = watch::channel(State::Running);
    let (_report, monitor) = watch::channel(None);
    assert_eq!(
        wait(status, monitor, async {
            anyhow::bail!("T1510 shutdown failure")
        })
        .await
        .unwrap_err()
        .to_string(),
        "T1510 shutdown failure"
    );
}
