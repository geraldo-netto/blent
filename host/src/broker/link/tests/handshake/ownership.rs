//! T1134: queue rejection preserves the exact original native connection.
use super::*;

#[tokio::test]
async fn t1134_full_and_closed_queues_return_original_connection_ownership() {
    for closed in [false, true] {
        rejected(closed).await;
    }
}

async fn rejected(closed: bool) {
    let (connection, mut link) = verified().await.prepared();
    let original = connection.completion();
    if closed {
        link.handle.stop();
        let _ = link.handle.next().await;
        completed(&link.handle.task_completion()).await;
    } else {
        let peer = pin(&settings().identity);
        for epoch in 1..=8 {
            assert!(link
                .handle
                .submit(Command::Approve(Ticket {
                    epoch,
                    peer: peer.clone(),
                }))
                .is_ok());
        }
    }
    let result = link.handle.submit(Command::Admit(Box::new(connection)));
    let rejected = result.is_err();
    let retained = !original.is_complete();
    let exact = if let Err(Command::Admit(mut returned)) = result {
        let pending = !returned.completion().is_complete();
        returned.retire().await.unwrap();
        pending && original.is_complete()
    } else {
        false
    };
    // Sponsor original cleanup before asserting, including faulty admission.
    link.handle.stop();
    let _ = link.handle.next().await;
    completed(&link.handle.task_completion()).await;
    drop(link);
    completed(&original).await;
    assert!(
        rejected && retained && exact,
        "T1134 rejection lost original native ownership"
    );
}
