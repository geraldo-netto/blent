//! T1684: both ready-branch orders preserve outcome and original cleanup.
use super::*;
use std::time::Duration;

async fn failed(panic: bool) -> EncoderTask {
    let handle = blent::owned_task::Task::spawn(async move {
        assert!(!panic, "T1684 reader panic");
        anyhow::bail!("T1684 malformed packet")
    })
    .unwrap();
    while !handle.is_finished() {
        tokio::task::yield_now().await;
    }
    EncoderTask {
        handle,
        idle: None,
        outcome: None,
    }
}
async fn selected(reader: &mut EncoderTask, joined_first: bool) -> Result<()> {
    if joined_first {
        reader.join().await;
    }
    reader.finish_session().await
}

#[tokio::test]
async fn t1684_reader_work_failure_restarts_in_both_ready_branch_orders() {
    for joined_first in [false, true] {
        let mut reader = failed(false).await;
        assert!(
            selected(&mut reader, joined_first).await.is_ok(),
            "T1684 work failure became fatal when joined_first={joined_first}"
        );
        assert!(reader.handle.is_joined());
    }
}

#[tokio::test]
async fn t1684_reader_panic_is_fatal_in_both_ready_branch_orders() {
    for joined_first in [false, true] {
        let mut reader = failed(true).await;
        assert!(
            selected(&mut reader, joined_first).await.is_err(),
            "T1684 panic erased when joined_first={joined_first}"
        );
    }
}

#[tokio::test]
async fn t1684_work_error_does_not_skip_original_idle_settlement() {
    let pool = blent_config::retirement::Pool::new(1).unwrap();
    let original = pool.reserve().unwrap();
    let ticket = original.completion();
    let mut reader = failed(false).await;
    reader.idle = Some(ticket.clone());
    assert!(
        tokio::time::timeout(Duration::from_millis(10), reader.retire())
            .await
            .is_err(),
        "T1684 work error bypassed original idle cleanup"
    );
    assert!(reader.handle.is_joined());
    assert!(!ticket.is_complete());
    drop(original);
    let failure = reader.retire().await.unwrap_err();
    assert!(failure.to_string().contains("T1684 malformed packet"));
    assert!(failure
        .chain()
        .any(|cause| cause.to_string() == "T1684 malformed packet"));
    assert!(ticket.is_complete());
}

#[tokio::test(start_paused = true)]
async fn t1684_retirement_deadline_is_fatal_and_retains_original_outcome() {
    for panic in [false, true] {
        let pool = blent_config::retirement::Pool::new(1).unwrap();
        let original = pool.reserve().unwrap();
        let ticket = original.completion();
        let mut reader = failed(panic).await;
        reader.idle = Some(ticket.clone());
        let error = reader.finish_session().await.unwrap_err();
        assert!(error.to_string().contains("retirement incomplete"));
        assert!(!ticket.is_complete());
        assert!(reader.outcome.as_ref().unwrap().is_err());
        drop(original);
        assert_eq!(reader.finish_session().await.is_err(), panic);
        assert!(ticket.is_complete());
    }
}

#[tokio::test]
async fn t1684_strict_cleanup_preserves_work_error_source_chain() {
    let mut reader = failed(false).await;
    let failure = reader.finish().await.unwrap_err();
    let tagged = failure.downcast_ref::<outcome::WorkFailure>().unwrap();
    assert!(std::error::Error::source(tagged)
        .unwrap()
        .to_string()
        .contains("T1684 malformed packet"));
    assert!(reader.finish().await.is_ok());
}
