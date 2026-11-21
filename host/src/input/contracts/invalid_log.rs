use super::*;

fn logging() -> (tempfile::NamedTempFile, tracing::subscriber::DefaultGuard) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(file.reopen().unwrap())
        .finish();
    (file, tracing::subscriber::set_default(subscriber))
}

#[tokio::test(start_paused = true)]
async fn t1501_invalid_frame_and_parser_error_have_bounded_log_output() {
    let (file, _logging) = logging();
    let controllers = Arc::new(Controllers::new(Arc::new(Recorder::default())));
    let lease = controllers.claim();
    let text = format!("{{\"type\":\"{}T1501-END\"}}", "x".repeat(60_000));
    assert!(
        dispatch_controller_text(
            &text,
            &controllers,
            lease.id,
            &Recorder::default(),
            &crate::latency::LatencyTracker::new(),
            true
        )
        .finish()
        .await
    );
    let output = std::fs::read_to_string(file.path()).unwrap();
    assert_eq!(output.matches("Invalid input").count(), 1);
    assert!(
        output.len() <= 1024,
        "T1501 invalid input emitted {} bytes",
        output.len()
    );
    assert!(!output.contains("T1501-END"));
}

#[tokio::test(start_paused = true)]
async fn t1501_invalid_input_bursts_are_counted_between_warning_windows() {
    let (file, _logging) = logging();
    let controllers = Arc::new(Controllers::new(Arc::new(Recorder::default())));
    let lease = controllers.claim();
    let settings = Recorder::default();
    let latency = crate::latency::LatencyTracker::new();
    for _ in 0..32 {
        assert!(
            dispatch_controller_text("{", &controllers, lease.id, &settings, &latency, true)
                .finish()
                .await
        );
    }
    assert_eq!(
        std::fs::read_to_string(file.path())
            .unwrap()
            .matches("Invalid input")
            .count(),
        1
    );
    // A replacement controller shares the listener's warning budget.
    drop(lease);
    let lease = controllers.claim();
    assert!(
        dispatch_controller_text("{", &controllers, lease.id, &settings, &latency, true)
            .finish()
            .await
    );
    assert_eq!(
        std::fs::read_to_string(file.path())
            .unwrap()
            .matches("Invalid input")
            .count(),
        1
    );
    tokio::time::advance(std::time::Duration::from_secs(1)).await;
    assert!(
        dispatch_controller_text("{", &controllers, lease.id, &settings, &latency, true)
            .finish()
            .await
    );
    let output = std::fs::read_to_string(file.path()).unwrap();
    assert_eq!(output.matches("Invalid input").count(), 2);
    assert!(output.contains("suppressed=32"), "{output}");
    assert!(
        dispatch_controller_text(
            r#"{"type":"mode","pen_only":true}"#,
            &controllers,
            lease.id,
            &settings,
            &latency,
            true
        )
        .finish()
        .await
    );
    assert!(settings
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|event| event.contains("mode true")));
}
