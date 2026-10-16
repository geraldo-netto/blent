use super::*;
use std::sync::mpsc;
fn limits() -> Limits {
    Limits {
        streams: [65536; 2],
        total: 65536,
    }
}
struct Held {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl Read for Held {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(0)
    }
}
fn completed(ticket: &Completion) {
    let end = Instant::now() + Duration::from_secs(2);
    while !ticket.is_complete() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn t815_held_collection_cannot_block_control_or_release_its_capacity() {
    for asynchronous in [false, true] {
        let pool = Pool::new(1).unwrap();
        let slot = pool.reserve().unwrap();
        let (started, entered) = mpsc::channel();
        let (release, hold) = mpsc::channel();
        let (returned, result) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let mut capture = CapturedOutput::start(
                    [
                        Box::new(Held {
                            entered: started,
                            release: hold,
                        }),
                        Box::new(io::empty()),
                    ],
                    limits(),
                    Instant::now() + Duration::from_millis(100),
                    slot,
                )
                .unwrap();
                let ticket = capture.completion.clone();
                capture.child_exited();
                let timed_out = if asynchronous {
                    tokio::time::timeout(Duration::from_millis(25), capture.result())
                        .await
                        .is_err()
                } else {
                    let status = Command::new(std::env::current_exe().unwrap())
                        .args(["--list"])
                        .stdout(Stdio::null())
                        .status()
                        .unwrap();
                    capture
                        .finish_sync(status, Instant::now() + Duration::from_millis(25))
                        .is_err_and(|error| error.kind() == io::ErrorKind::TimedOut)
                };
                returned.send((timed_out, ticket)).unwrap();
            });
        });
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let response = result.recv_timeout(Duration::from_millis(200));
        let held = pool.reserve().is_err();
        let incomplete = response
            .as_ref()
            .is_ok_and(|(_, ticket)| !ticket.is_complete());
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(
            response.is_ok(),
            "T815: held collection blocked control executor"
        );
        let (timed_out, ticket) = response.unwrap();
        assert!(timed_out && held && incomplete);
        completed(&ticket);
        assert_eq!(pool.active(), 0);
    }
}

struct Interrupted(bool);
impl Read for Interrupted {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !std::mem::replace(&mut self.0, true) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        buffer[0] = 7;
        Ok(1)
    }
}
#[test]
fn t815_interrupted_reads_retry_without_losing_budget_or_bytes() {
    let mut source = Interrupted(false);
    let mut closed = false;
    let mut scratch = [0; 1];
    let mut bytes = Bytes::new(limits());
    assert!(poll_source(&mut source, &mut closed, &mut scratch, &mut bytes, 0).unwrap());
    assert!(bytes.streams[0].is_empty());
    assert!(poll_source(&mut source, &mut closed, &mut scratch, &mut bytes, 0).unwrap());
    assert_eq!(bytes.streams[0], vec![7]);
}
struct Closing {
    state: Arc<AtomicU8>,
    read: usize,
}
impl Read for Closing {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.read += 1;
        match self.read {
            1 => {
                self.state.store(FINISHED, Ordering::Release);
                Err(io::ErrorKind::WouldBlock.into())
            }
            2 => {
                buffer[0] = 9;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}
#[test]
fn t815_child_exit_after_idle_probe_still_drains_final_bytes() {
    let state = Arc::new(AtomicU8::new(RUNNING));
    let bytes = collect(
        [
            Box::new(Closing {
                state: state.clone(),
                read: 0,
            }),
            Box::new(io::empty()),
        ],
        limits(),
        Instant::now() + Duration::from_secs(1),
        &state,
    )
    .unwrap();
    assert_eq!(bytes, [vec![9], vec![]]);
}
struct Broken(bool);
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        assert!(!self.0, "T815 native reader panic");
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
#[tokio::test]
async fn t815_collection_failure_panic_and_cancellation_retire_owners() {
    for panic in [false, true] {
        let pool = Pool::new(1).unwrap();
        let mut capture = CapturedOutput::start(
            [Box::new(Broken(panic)), Box::new(io::empty())],
            limits(),
            Instant::now() + Duration::from_secs(1),
            pool.reserve().unwrap(),
        )
        .unwrap();
        let ticket = capture.completion.clone();
        let result = capture.result().await.unwrap_err();
        assert_eq!(
            result.kind(),
            if panic {
                io::ErrorKind::BrokenPipe
            } else {
                io::ErrorKind::PermissionDenied
            }
        );
        drop(capture);
        completed(&ticket);
    }
    for state in [CANCELLED, RUNNING] {
        let result = collect(
            [Box::new(io::empty()), Box::new(io::empty())],
            limits(),
            Instant::now(),
            &AtomicU8::new(state),
        );
        assert_eq!(
            result.unwrap_err().kind(),
            if state == CANCELLED {
                io::ErrorKind::Interrupted
            } else {
                io::ErrorKind::TimedOut
            }
        );
    }
    let state = Arc::new(AtomicU8::new(RUNNING));
    let (sender, receiver) = oneshot::channel();
    drop(sender);
    let pool = Pool::new(1).unwrap();
    let slot = pool.reserve().unwrap();
    let mut capture = CapturedOutput {
        state,
        receiver,
        completion: slot.completion(),
    };
    assert_eq!(
        capture.poll_result().unwrap().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn t1031_held_collector_reports_exact_original_thread_until_join() {
    let slot = Pool::new(1).unwrap().reserve().unwrap();
    let original = slot.completion();
    let (started, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let capture = CapturedOutput::start(
        [
            Box::new(Held {
                entered: started,
                release: held,
            }),
            Box::new(io::empty()),
        ],
        limits(),
        Instant::now() + Duration::from_secs(5),
        slot,
    )
    .unwrap();
    let observed = std::sync::Mutex::new(Vec::new());
    capture.observe(&|proof| observed.lock().unwrap().push(proof));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(capture);
    let originals = observed.into_inner().unwrap();
    let retained = originals.iter().any(|proof| !proof.is_complete());
    assert!(!original.is_complete());
    release.send(()).unwrap();
    completed(&original);
    assert_eq!(originals.len(), 1, "T1031 missing original held collector");
    assert!(
        retained,
        "T1031 collector cancellation replaced actual thread join"
    );
    assert!(originals[0].is_complete());
}
