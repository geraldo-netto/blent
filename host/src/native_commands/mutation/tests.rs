use super::*;
use std::os::unix::process::ExitStatusExt;
#[test]
fn t1444_native_exit_and_missing_ack_are_distinct_from_no_effect() {
    for reported in [0, 1, 2, 3, usize::MAX] {
        for status in [0, 7 << 8, libc::SIGTERM] {
            let output = Ok(Output {
                status: std::process::ExitStatus::from_raw(status),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
            assert_eq!(
                classify(&output, reported) == Outcome::Settled,
                reported < 2 || (reported == 2 && status == 0)
            );
        }
        assert_eq!(
            classify(&Err(io::ErrorKind::BrokenPipe.into()), reported) == Outcome::Settled,
            reported < 2
        );
    }
}

#[tokio::test]
async fn t1444_mutation_worker_panic_is_unknown_even_without_native_debt() {
    let outcome = Arc::new(Mutex::new(Outcome::Pending));
    let worker = tokio::spawn(async {
        panic!("T1444 original worker panic");
    });
    while !worker.is_finished() {
        tokio::task::yield_now().await;
    }
    let mut operation = Operation {
        worker: Some(worker),
        outcome: outcome.clone(),
    };
    assert!(!operation.complete());
    assert_eq!(*outcome.lock().unwrap(), Outcome::Unknown);
    assert!(!operation.complete());
}
