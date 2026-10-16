use super::*;
use std::{
    os::fd::{AsRawFd, IntoRawFd},
    time::Duration,
};
fn job() -> Job {
    Job {
        encoder: "libx264".into(),
        width: 64,
        height: 64,
        fps: 60,
        bitrate: 1000,
        quality: 20,
        slots: 4,
        workers: 1,
        device: "/dev/dri/renderD128".into(),
        nonce: nonce().unwrap(),
        parent: std::process::id(),
    }
}
fn output() -> (
    Output,
    tokio::sync::broadcast::Receiver<crate::media::VideoPacket>,
) {
    let (video, receiver) = crate::video_queue::channel(8, Default::default());
    (
        Output {
            retirement: None,
            video: video.into(),
            headers: Default::default(),
            latency: Default::default(),
            refresh: Default::default(),
        },
        receiver,
    )
}
#[test]
fn t819_worker_arguments_reject_invalid_job_flags_and_descriptors() {
    assert!(dispatch(Vec::new().into_iter()).is_none());
    assert!(dispatch(vec![OsString::from("--status")].into_iter()).is_none());
    assert!(dispatch(vec![OsString::from(wire::ARGUMENT)].into_iter())
        .unwrap()
        .is_err());
    let encoded = job().encode().unwrap();
    assert!(arguments(vec![OsString::from("{}")].into_iter()).is_err());
    assert!(arguments(vec![OsString::from(&encoded)].into_iter()).is_err());
    for extra in [
        vec!["bad"],
        vec!["--capture-socket-fd"],
        vec!["--capture-socket-fd", "-1"],
        vec!["--capture-socket-fd", "0"],
        vec!["--capture-socket-fd", "NaN"],
    ] {
        let mut args = vec![OsString::from(&encoded), OsString::from("/unused")];
        args.extend(extra.into_iter().map(OsString::from));
        assert!(arguments(args.into_iter()).is_err());
    }
    assert!(
        arguments(vec![OsString::from(&encoded), OsString::from("/unused")].into_iter()).is_ok()
    );
    let (socket, _peer) = crate::raw_socket::Socket::pair().unwrap();
    let fd = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
    let args = vec![
        OsString::from(&encoded),
        OsString::from("/unused"),
        OsString::from("--capture-socket-fd"),
        OsString::from(fd.to_string()),
    ];
    assert!(arguments(args.into_iter()).unwrap().2.is_some());
    let file = tempfile::tempfile().unwrap();
    assert!(crate::raw_socket::Socket::from_inherited(file.into_raw_fd()).is_err());
}
#[test]
fn t819_controls_packet_writer_and_activity_preserve_bounds() {
    for bytes in [b"I".as_slice(), b"", b"X", b"IIX"] {
        let refresh = Arc::new(AtomicBool::new(false));
        let stop = crate::encoder_io::StopSignal::new().unwrap();
        control(bytes, refresh.clone(), stop.clone());
        assert!(stop.requested());
        assert_eq!(refresh.load(Ordering::Acquire), bytes.starts_with(b"I"));
    }
    let mut bytes = Vec::new();
    write_packet(&mut bytes, b"data", true).unwrap();
    assert_eq!(bytes, [0, 0, 0, 4, 1, b'd', b'a', b't', b'a']);
    assert!(write_packet(&mut bytes, b"", false).is_err());
    assert!(write_packet(&mut std::io::Cursor::new([0; 2]), b"data", false).is_err());
    use blent::capture_contract::PacketOutput;
    let pipe = Pipe {
        bytes: Mutex::new(tempfile::tempfile().unwrap()),
        stop: crate::encoder_io::StopSignal::new().unwrap(),
    };
    assert_eq!(pipe.receiver_count(), 1);
    assert_eq!(pipe.viewers().epoch(), 0);
}
#[tokio::test]
async fn t819_reader_rejects_truncated_and_oversized_output_before_publication() {
    for bytes in [
        vec![],
        vec![0, 0],
        vec![0, 0, 0, 0, 0],
        vec![255, 255, 255, 255, 0],
        vec![0, 0, 0, 4, 1, 7],
    ] {
        let (output, _receiver) = output();
        assert!(packets(&job(), bytes.as_slice(), output).await.is_err());
    }
    let (output, mut receiver) = output();
    let config = output.headers.clone();
    let data = [
        0, 0, 0, 1, 0x67, 0x42, 0, 0x1e, 0, 0, 0, 1, 0x68, 0xce, 0x06, 0xe2, 0, 0, 0, 1, 0x65,
        0x88, 0x80,
    ];
    let mut bytes = Vec::new();
    write_packet(&mut bytes, &data, true).unwrap();
    assert!(packets(&job(), bytes.as_slice(), output).await.is_err());
    let packet = receiver.recv().await.unwrap();
    assert_eq!(&*packet.data, &data);
    assert!(!packet.generation.load(Ordering::Acquire));
    assert!(config.current().is_some());
}
#[tokio::test]
async fn t819_shared_worker_death_hang_restart_and_final_lease_retirement() {
    let mut producer = crate::encoder::raw_tests::Producer::new();
    for signal in [libc::SIGSTOP, libc::SIGKILL] {
        let previous = producer.command("generation");
        let (output, mut receiver) = output();
        let (child, mut task) = spawn(
            job(),
            Path::new("/unused-shared-fifo"),
            Some(producer.socket.clone()),
            output,
            (),
        )
        .unwrap();
        let end = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            assert!(
                std::time::Instant::now() < end,
                "T819 worker failed to request shared input"
            );
            assert_eq!(producer.command("service"), 1);
            if producer.command("generation") > previous {
                assert_eq!(producer.command("frame 96"), 1);
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let packet = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(packet.is_idr);
        assert!(packet.codec_config.is_some());
        let pid = child.id().unwrap();
        assert_eq!(unsafe { libc::kill(pid as i32, signal) }, 0);
        let retired = child.retire(Duration::from_millis(20)).await;
        assert!(
            retired.completion.is_complete(),
            "T819 child retirement not confirmed"
        );
        assert!(tokio::time::timeout(Duration::from_secs(2), &mut task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        assert!(!packet.generation.load(Ordering::Acquire));
        // Parent and producer remain alive; successor performs a fresh nonce handshake.
    }
}

fn cpu(which: i32) -> f64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::getrusage(which, &mut usage) }, 0);
    (usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) as f64 * 1000.0
        + (usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) as f64 / 1000.0
}
fn resident(pid: u32) -> u64 {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .unwrap()
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}
async fn handshake(producer: &mut crate::encoder::raw_tests::Producer, generation: i32) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        assert!(std::time::Instant::now() < end);
        assert_eq!(producer.command("service"), 1);
        if producer.command("generation") > generation {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn measure(
    producer: &mut crate::encoder::raw_tests::Producer,
    isolated: bool,
    width: u32,
    height: u32,
    frames: usize,
) -> (serde_json::Value, String) {
    use sha2::Digest;
    let mut profile = job();
    profile.width = width;
    profile.height = height;
    let generation = producer.command("generation");
    let start_cpu = cpu(libc::RUSAGE_SELF) + cpu(libc::RUSAGE_CHILDREN);
    let started = std::time::Instant::now();
    let (output, mut receiver) = output();
    let mut child = None;
    let mut task = None;
    let mut native = None;
    let stop = crate::encoder_io::StopSignal::new().unwrap();
    if isolated {
        let (process, work) = spawn(
            profile.clone(),
            Path::new("/unused"),
            Some(producer.socket.clone()),
            output,
            (),
        )
        .unwrap();
        child = Some(process);
        task = Some(work);
    } else {
        let socket = producer.socket.clone();
        let stopped = stop.clone();
        native = Some(std::thread::spawn(move || {
            crate::encoder::run(
                Path::new("/unused"),
                "libx264",
                width,
                height,
                60,
                1000,
                20,
                output.video,
                output.headers,
                Arc::new(AtomicBool::new(true)),
                stopped,
                output.latency,
                Some(socket),
                4,
                1,
                "/dev/dri/renderD128",
            )
        }));
    }
    handshake(producer, generation).await;
    let startup_us = started.elapsed().as_micros() as u64;
    let mut timings = Vec::new();
    let mut hash = sha2::Sha256::new();
    for index in 0..frames {
        let start = std::time::Instant::now();
        assert_eq!(producer.command(&format!("frame {}", 64 + index % 32)), 1);
        let packet = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        if index >= frames.min(30) / 2 {
            timings.push(start.elapsed().as_micros() as u64);
        }
        hash.update(&*packet.data);
    }
    let parent_rss = resident(std::process::id());
    let worker_rss = child
        .as_ref()
        .map(|c| resident(c.id().unwrap()))
        .unwrap_or(0);
    if let Some(mut work) = task {
        work.abort();
        let _ = (&mut work).await;
    }
    stop.request();
    if let Some(thread) = native {
        thread.join().unwrap().unwrap();
    }
    if let Some(process) = child {
        assert!(process
            .retire(Duration::from_secs(1))
            .await
            .completion
            .is_complete());
    }
    let elapsed_cpu = cpu(libc::RUSAGE_SELF) + cpu(libc::RUSAGE_CHILDREN) - start_cpu;
    timings.sort_unstable();
    let measured = serde_json::json!({"isolated": isolated, "width": width, "height": height, "frames": frames,
        "startup_us": startup_us, "parent_rss_kib": parent_rss, "worker_rss_kib": worker_rss,
        "parent_encoder_cpu_ms": elapsed_cpu, "p50_us": timings[timings.len()/2], "p95_us": timings[timings.len()*95/100]});
    (measured, format!("{:x}", hash.finalize()))
}
#[tokio::test]
async fn t819_worker_matches_inprocess_shared_output_with_paired_measurements() {
    let dimensions = std::env::var("BLENT_T819_MEASURE").ok();
    let (width, height, frames, pairs) = dimensions
        .map(|s| {
            let (w, h) = s.split_once('x').unwrap();
            (w.parse().unwrap(), h.parse().unwrap(), 150, 5)
        })
        .unwrap_or((64, 64, 3, 1));
    let mut producer = crate::encoder::raw_tests::Producer::new();
    assert_eq!(producer.command(&format!("resize {width} {height}")), 1);
    let mut results = Vec::new();
    for pair in 0..pairs {
        let order = if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        let first = measure(&mut producer, order[0], width, height, frames).await;
        let second = measure(&mut producer, order[1], width, height, frames).await;
        assert_eq!(
            first.1, second.1,
            "T819 isolated encoder changed complete shared-input bitstream"
        );
        results.push(first.0);
        results.push(second.0);
    }
    println!("T819_MEASURE {}", serde_json::to_string(&results).unwrap());
}

#[test]
fn t819_packet_writer_retries_interruption_and_preserves_failed_packet_ownership() {
    struct Interrupted {
        first: bool,
        data: Vec<u8>,
    }
    impl Write for Interrupted {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.first {
                self.first = false;
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            self.data.extend_from_slice(&bytes[..1]);
            Ok(1)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Interrupted {
        first: true,
        data: Vec::new(),
    };
    write_packet(&mut output, b"abc", false).unwrap();
    assert_eq!(output.data, [0, 0, 0, 3, 0, b'a', b'b', b'c']);
    use blent::capture_contract::PacketOutput;
    let stop = crate::encoder_io::StopSignal::new().unwrap();
    let pipe = Pipe {
        bytes: Mutex::new(tempfile::tempfile().unwrap()),
        stop: stop.clone(),
    };
    let packet = crate::media::VideoPacket {
        data: Vec::<u8>::new().into(),
        is_idr: true,
        seq: 7,
        codec_config: None,
        generation: Arc::new(AtomicBool::new(true)),
    };
    assert_eq!(pipe.send(packet).err().unwrap().seq, 7);
    assert!(stop.requested());
}
#[tokio::test]
async fn t819_parent_identity_failure_and_absent_executable_propagate() {
    let (output, _receiver) = output();
    assert!(spawn_at(
        Path::new("/missing-t819-worker"),
        job(),
        Path::new("/unused"),
        None,
        output,
        ()
    )
    .is_err());
    let (output, _receiver) = self::output();
    let mut wrong = job();
    wrong.parent = u32::MAX;
    let (child, mut task) = spawn(wrong, Path::new("/unused"), None, output, ()).unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(2), &mut task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(child
        .retire(Duration::from_millis(20))
        .await
        .completion
        .is_complete());
}

// T1028: report the actual child and reader, including retained native leases.
#[derive(Default)]
struct Retained(Mutex<Vec<blent_config::retirement::Completion>>);
impl blent::capture_contract::NativeRetirement for Retained {
    fn retain(&self, original: blent_config::retirement::Completion) {
        self.0.lock().unwrap().push(original);
    }
}
struct HeldCleanup {
    entered: std::sync::mpsc::Sender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl Drop for HeldCleanup {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
    }
}
struct Release(std::sync::mpsc::Sender<()>);
impl Drop for Release {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[tokio::test]
async fn t1028_isolated_encoder_reports_original_child_and_reader() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("worker");
    std::fs::write(&script, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let report = Arc::new(Retained::default());
    let (mut output, _receiver) = output();
    output.retirement = Some(report.clone());
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = Release(release);
    let (mut child, task) = spawn_at(
        &script,
        job(),
        &root.path().join("fifo"),
        None,
        output,
        HeldCleanup {
            entered: entered_tx,
            release: Mutex::new(held),
        },
    )
    .unwrap();
    let original = child.completion();
    let reader = task.completion();
    let recorded = report.0.lock().unwrap().clone();
    child.start_kill().unwrap();
    drop(child);
    task.abort();
    drop(task);
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!original.is_complete());
    let held_recorded = recorded.iter().any(|proof| !proof.is_complete());
    drop(release);
    for proof in [&original, &reader] {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !proof.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    assert_eq!(recorded.len(), 2, "T1028 missing original encoder proofs");
    assert!(held_recorded, "T1028 original child cleanup not retained");
    assert!(recorded.iter().all(|proof| proof.is_complete()));
}

#[tokio::test]
async fn t1028_failed_reader_admission_keeps_original_child_proof() {
    if std::env::var_os("BLENT_T1028_WORKER_LIMIT").is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "encoder_worker::tests::t1028_failed_reader_admission_keeps_original_child_proof",
                "--nocapture",
            ])
            .env("BLENT_T1028_WORKER_LIMIT", "1")
            .env("BLENT_TASK_LIMIT", "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("worker");
    std::fs::write(&script, "#!/bin/sh\nexec sleep 60\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let blocker = blent::owned_task::Task::spawn(std::future::pending()).unwrap();
    let report = Arc::new(Retained::default());
    let (mut output, _receiver) = output();
    output.retirement = Some(report.clone());
    let (entered_tx, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel();
    let release = Release(release);
    assert!(spawn_at(
        &script,
        job(),
        &root.path().join("fifo"),
        None,
        output,
        HeldCleanup {
            entered: entered_tx,
            release: Mutex::new(held)
        }
    )
    .is_err());
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let originals = report.0.lock().unwrap().clone();
    let pending = originals.iter().any(|proof| !proof.is_complete());
    drop(release);
    for proof in &originals {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !proof.is_complete() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    drop(blocker);
    assert_eq!(
        originals.len(),
        1,
        "T1028 post-spawn failure lost original child"
    );
    assert!(pending, "T1028 failed reader erased held native cleanup");
}
