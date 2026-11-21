//! T1509: pending original retirement remains visible without releasing its debt.
use super::*;
use std::io::Write;
use std::sync::Arc;

#[derive(Clone)]
struct Log(Arc<Mutex<Vec<u8>>>);
impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn t1509_drain_reports_original_pending_locations_until_settlement() {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let sink = bytes.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || Log(sink.clone()))
        .finish();
    let _logging = tracing::subscriber::set_default(subscriber);
    let registry = Registry::default();
    let pool = Pool::new(1).unwrap();
    let original = registry.reserve(&pool).unwrap();
    let draining = registry.drain();
    tokio::pin!(draining);
    assert!(futures_util::poll!(&mut draining).is_pending());
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert!(futures_util::poll!(&mut draining).is_pending());
    assert!(!original.completion().is_complete());
    drop(original);
    draining.await;
    let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert_eq!(
        text.matches("Waiting for native retirement").count(),
        2,
        "T1509 exit drain was silent: {text}"
    );
    assert!(
        text.contains("pending=1") && text.contains("diagnostic_tests.rs"),
        "{text}"
    );
}
