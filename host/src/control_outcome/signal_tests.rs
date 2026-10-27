//! T1129: signal-registration errors retain original stop/settlement obligations.
use super::*;
use futures_util::FutureExt;
use tokio::sync::{oneshot, watch};

#[tokio::test]
async fn t1129_registration_failure_cannot_escape_original_native_cleanup() {
    for work_failed in [false, true] {
        signal_outcome(Trigger::Interrupt(true), work_failed).await;
    }
}
#[tokio::test]
async fn t1129_normal_signal_preserves_work_result_after_actual_settlement() {
    for work_failed in [false, true] {
        signal_outcome(Trigger::Interrupt(false), work_failed).await;
    }
}

#[derive(Clone, Copy)]
enum Trigger {
    Interrupt(bool),
    Terminate,
}

#[tokio::test]
async fn t1129_termination_waits_for_original_cleanup_and_keeps_work_errors() {
    for work_failed in [false, true] {
        signal_outcome(Trigger::Terminate, work_failed).await;
    }
}

async fn signal_outcome(trigger: Trigger, work_failed: bool) {
    let (stop, mut stopped) = watch::channel(false);
    let (release, released) = tokio::sync::oneshot::channel();
    let owner = blent_config::retirement::Pool::new(1)
        .unwrap()
        .reserve()
        .unwrap();
    let proof = owner.completion();
    let mut work = Box::pin(async move {
        let _original = owner;
        stopped.wait_for(|value| *value).await.unwrap();
        released.await.unwrap();
        if work_failed {
            anyhow::bail!("T1129 original work failed");
        }
        Ok(())
    });
    let (interrupt, interrupted) = oneshot::channel();
    let (terminate, terminated) = oneshot::channel();
    let mut interrupt_sender = Some(interrupt);
    let mut terminate_sender = Some(terminate);
    let mut finishing = Box::pin(with_signals(
        work.as_mut(),
        &stop,
        async { interrupted.await.unwrap() },
        async { terminated.await.unwrap() },
    ));
    assert!(finishing.as_mut().now_or_never().is_none());
    fire(trigger, &mut interrupt_sender, &mut terminate_sender);
    let early = finishing.as_mut().now_or_never();
    let requested_stop = *stop.borrow();
    let returned_early = early.is_some();
    // Sponsor exact original cleanup even when the old branch returns early.
    stop.send_replace(true);
    release.send(()).unwrap();
    let result = match early {
        Some(result) => {
            drop(finishing);
            let _ = work.await;
            result
        }
        None => finishing.await,
    };
    assert!(proof.is_complete());
    check_result(
        result,
        matches!(trigger, Trigger::Interrupt(true)),
        work_failed,
    );
    assert!(
        requested_stop,
        "T1129 failed signal did not request original stop"
    );
    assert!(
        !returned_early,
        "T1129 failed signal escaped before original retirement"
    );
}

fn fire(
    trigger: Trigger,
    interrupt: &mut Option<oneshot::Sender<std::io::Result<()>>>,
    terminate: &mut Option<oneshot::Sender<()>>,
) {
    match trigger {
        Trigger::Interrupt(failed) => {
            let signal = if failed {
                Err(std::io::Error::other("T1129 signal registration failed"))
            } else {
                Ok(())
            };
            interrupt.take().unwrap().send(signal).unwrap();
        }
        Trigger::Terminate => terminate.take().unwrap().send(()).unwrap(),
    }
}

#[tokio::test]
async fn t1129_completed_work_does_not_request_an_unrelated_stop() {
    for work_failed in [false, true] {
        let (stop, _) = watch::channel(false);
        let mut work = Box::pin(async {
            if work_failed {
                anyhow::bail!("original work failed");
            }
            Ok(())
        });
        let result = with_signals(
            work.as_mut(),
            &stop,
            std::future::pending(),
            std::future::pending(),
        )
        .await;
        check_result(result, false, work_failed);
        assert!(!*stop.borrow());
    }
}

#[tokio::test]
async fn t1129_media_cli_entrypoints_validate_without_native_devices() {
    let camera = blent_config::camera::CameraOptions {
        profile: blent_config::camera::CameraProfile {
            width: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(crate::camera::run(&camera).await.is_err());
    let mut audio =
        blent_config::audio::AudioOptions::new(blent_config::audio::Direction::Speakers);
    audio.profile.buffer_ms = 0;
    assert!(crate::audio::run_cli(audio).await.is_err());
}
fn check_result(result: Result<()>, signal_failed: bool, work_failed: bool) {
    let failed = signal_failed || work_failed;
    assert_eq!(result.is_err(), failed);
    if signal_failed {
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("signal registration failed"));
    } else if work_failed {
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("original work failed"));
    }
}
