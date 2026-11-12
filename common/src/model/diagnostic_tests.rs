use super::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn t1359_bitrate_diagnostics_describe_policy_in_both_directions() {
    for (requested, selected) in [(0, 1000), (999, 1000), (60001, 60000), (u32::MAX, 60000)] {
        let log = Log::default();
        let writer = log.clone();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let mut config = FileConfig {
            bitrate: requested,
            ..Default::default()
        };
        tracing::subscriber::with_default(subscriber, || config.sanitize_requested());
        assert_eq!(config.bitrate, selected);
        let message = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
        assert!(message.contains(&format!("Bitrate {requested} kbps is outside the configured range 1000–60000 kbps — clamped to {selected} kbps")), "T1359: {message}");
        assert!(
            !message.contains("USB"),
            "T1359 policy is not measured link capacity"
        );
    }
}
