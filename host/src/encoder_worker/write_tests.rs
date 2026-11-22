//! T1492: packet failures remain terminal and reach the worker exit status.
use super::*;
use blent::capture_contract::PacketOutput;
use std::time::Duration;

fn packet(bytes: usize, seq: u32) -> crate::media::VideoPacket {
    crate::media::VideoPacket {
        data: vec![1; bytes].into(),
        is_idr: true,
        seq,
        codec_config: None,
        generation: Arc::new(AtomicBool::new(true)),
    }
}

#[test]
fn t1492_invalid_packet_permanently_stops_the_original_output() {
    for size in [wire::MAX_PACKET + 1, 0] {
        let file = tempfile::tempfile().unwrap();
        let observed = file.try_clone().unwrap();
        let pipe = Pipe::new(file, crate::encoder_io::StopSignal::new().unwrap());
        assert_eq!(pipe.send(packet(size, 4)).err().unwrap().seq, 4);
        assert!(pipe.stop.requested());
        let second = pipe.send(packet(16, 5));
        assert!(
            second.is_err(),
            "T1492 output resumed after its original packet failure"
        );
        assert_eq!(second.err().unwrap().seq, 5);
        assert_eq!(observed.metadata().unwrap().len(), 0);
        assert_eq!(
            format!("{:#}", pipe.finish(Ok(())).unwrap_err()),
            "Encoder packet output failed: Invalid encoded packet size"
        );
    }
}

#[test]
fn t1492_first_io_failure_survives_later_packet_and_encoder_errors() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let pipe = Pipe::new(
        std::fs::File::open(file.path()).unwrap(),
        crate::encoder_io::StopSignal::new().unwrap(),
    );
    assert_eq!(pipe.send(packet(16, 6)).err().unwrap().seq, 6);
    assert!(pipe.stop.requested());
    assert_eq!(pipe.send(packet(0, 7)).err().unwrap().seq, 7);
    let error = pipe
        .finish(Err(anyhow::anyhow!("later encoder error")))
        .unwrap_err();
    assert_eq!(error.to_string(), "Encoder packet output failed");
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::EBADF)
    );
    assert_eq!(file.as_file().metadata().unwrap().len(), 0);
}

#[test]
fn t1492_successful_output_preserves_packet_bytes_and_encoder_result() {
    for failure in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let pipe = Pipe::new(
            file.reopen().unwrap(),
            crate::encoder_io::StopSignal::new().unwrap(),
        );
        assert_eq!(pipe.send(packet(3, 8)).ok(), Some(1));
        assert!(!pipe.stop.requested());
        let result = if failure {
            Err(anyhow::anyhow!("encoder failure"))
        } else {
            Ok(())
        };
        let result = pipe.finish(result);
        assert_eq!(
            std::fs::read(file.path()).unwrap(),
            [0, 0, 0, 3, 1, 1, 1, 1]
        );
        assert_eq!(
            result.err().map(|error| error.to_string()),
            failure.then(|| "encoder failure".to_owned())
        );
    }
}

#[test]
fn t1492_completion_requires_exclusive_unpoisoned_output() {
    let pipe = Pipe::new(
        tempfile::tempfile().unwrap(),
        crate::encoder_io::StopSignal::new().unwrap(),
    );
    let retained = pipe.clone();
    assert_eq!(
        pipe.finish(Ok(())).unwrap_err().to_string(),
        "Encoder packet output still owned"
    );
    retained.finish(Ok(())).unwrap();
    let pipe = Pipe::new(
        tempfile::tempfile().unwrap(),
        crate::encoder_io::StopSignal::new().unwrap(),
    );
    let writer = pipe.clone();
    assert!(std::thread::spawn(move || {
        let _guard = writer.state.lock().unwrap();
        panic!("T1492 deliberate writer poison");
    })
    .join()
    .is_err());
    assert_eq!(
        pipe.finish(Ok(())).unwrap_err().to_string(),
        "Encoder packet output poisoned"
    );
}

#[tokio::test]
async fn t1492_native_worker_reports_packet_write_failure() {
    let mut producer = crate::encoder::raw_tests::Producer::new();
    let generation = producer.command("generation");
    let profile = tests::job();
    let executable = test_executable(&std::env::current_exe().unwrap()).unwrap();
    let errors = tempfile::NamedTempFile::new().unwrap();
    let mut command = tokio::process::Command::new(executable);
    command.args([wire::ARGUMENT, &profile.encode().unwrap(), "/unused"]);
    command.stdin(std::process::Stdio::piped());
    command.stdout(
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap(),
    );
    command.stderr(errors.reopen().unwrap());
    producer.socket.attach(&mut command).unwrap();
    let child = blent_config::commands::OwnedChild::spawn(&mut command).unwrap();
    let completion = child.completion();
    tests::handshake(&mut producer, generation).await;
    assert_eq!(producer.command("frame 96"), 1);
    let status = child.finish(Duration::from_secs(5)).await.unwrap();
    assert!(completion.is_complete());
    let error = std::fs::read_to_string(errors.path()).unwrap();
    assert!(
        !status.success(),
        "T1492 worker hid its packet write failure: {error}"
    );
    assert!(error.contains("Encoder packet output failed"), "{error}");
    assert!(error.contains("No space left on device"), "{error}");
}
