//! T1236: native failure diagnostics retain only bounded structural metadata.
use super::*;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut retained = self.0.lock().unwrap();
        assert!(retained.len() + bytes.len() <= 4096);
        retained.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn t1236_native_link_failure_logs_identity_and_kind_without_payload() {
    let mut fixture = Fixture::new().await;
    let capture = Capture::default();
    let output = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || output.clone())
        .finish();
    let error = io::Error::new(
        io::ErrorKind::ConnectionReset,
        "T1236_SECRET_NATIVE_PAYLOAD",
    );
    let result = tracing::subscriber::with_default(subscriber, || {
        fixture.server.input(Input::Link(999, Err(error.into())))
    });
    fixture.finish().await;
    let text = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(result.is_err(), "unknown link must remain rejected");
    assert!(
        text.contains("USB link failed"),
        "T1236 missing native diagnostic: {text}"
    );
    assert!(text.contains("connection=999"));
    assert!(text.contains("ConnectionReset"));
    assert!(!text.contains("T1236_SECRET_NATIVE_PAYLOAD"));
}

#[test]
fn t1236_phase_labels_are_fixed_public_metadata() {
    use link::FailureStage::*;
    for (phase, expected) in [
        (Opening, "Opening"),
        (Discovery, "Discovery"),
        (Authentication, "Authentication"),
        (Window, "Window"),
        (Media, "Media"),
        (Control, "Control"),
    ] {
        assert_eq!(phase.to_string(), expected);
    }
}
