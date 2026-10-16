//! T1022: independent standard-line oracle and bounded malformed input properties.
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn bounded<R: AsyncRead + Unpin>(lines: &Lines<R>) {
    assert!(lines.bytes.len() <= MAX_RECORD_BYTES);
    assert!(lines.bytes.capacity() <= MAX_RECORD_BYTES);
    assert!(lines.reader.buffer().len() <= MAX_RECORD_BYTES);
}

#[tokio::test]
async fn t1022_all_record_extents_are_bounded_before_accumulation() {
    for length in 0..=MAX_RECORD_BYTES + 2 {
        for terminated in [false, true] {
            let mut input = vec![b'x'; length];
            if terminated {
                input.push(b'\n');
            }
            let mut lines = Lines::new(input.as_slice());
            let result = lines.next_line().await;
            assert_eq!(result.is_ok(), input.len() <= MAX_RECORD_BYTES);
            bounded(&lines);
            if result.is_err() {
                assert!(lines.next_line().await.is_err());
            }
        }
    }
}

async fn standard_oracle(input: &[u8]) {
    let mut expected = BufReader::new(input).lines();
    let mut actual = Lines::new(input);
    loop {
        let expected = expected.next_line().await;
        let result = actual.next_line().await;
        bounded(&actual);
        assert_eq!(result.is_ok(), expected.is_ok());
        match expected {
            Ok(line) => {
                assert_eq!(result.unwrap(), line);
                if line.is_none() {
                    break;
                }
            }
            Err(_) => {
                assert!(actual.next_line().await.is_err());
                break;
            }
        }
    }
}

#[tokio::test]
async fn t1022_utf8_crlf_blank_and_partial_eof_match_standard_lines() {
    for input in [
        b"".as_slice(),
        b"\n",
        b"\r",
        b"\r\n",
        b"a\nb\r\n\nc\r",
        "é\r\nλ".as_bytes(),
    ] {
        standard_oracle(input).await;
    }
    for byte in 0..=255 {
        standard_oracle(&[b'A', byte, b'\n', b'B']).await;
    }
}

#[tokio::test]
async fn t1022_canceled_line_retains_original_partial_bytes() {
    let (mut writer, reader) = tokio::io::duplex(2048);
    let mut lines = Lines::new(reader);
    writer.write_all(b"MODE_").await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), lines.next_line())
            .await
            .is_err()
    );
    assert_eq!(lines.bytes, b"MODE_");
    bounded(&lines);
    writer.write_all(b"CHANGED 8 8 60\r\n").await.unwrap();
    assert_eq!(
        lines.next_line().await.unwrap().as_deref(),
        Some("MODE_CHANGED 8 8 60")
    );
    writer
        .write_all(&vec![b'x'; MAX_RECORD_BYTES])
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), lines.next_line())
            .await
            .is_err()
    );
    bounded(&lines);
    writer
        .write_all(b"\nMODE_CHANGED 16 16 60\n")
        .await
        .unwrap();
    assert!(lines.next_line().await.is_err());
    bounded(&lines);
    assert!(lines.next_line().await.is_err());
}

struct ErrorReader(Arc<AtomicUsize>);

impl AsyncRead for ErrorReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        _: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(Err(std::io::Error::other("T1022 read failed")))
    }
}

#[tokio::test]
async fn t1022_native_read_error_is_terminal_without_repolling() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut lines = Lines::new(ErrorReader(calls.clone()));
    assert!(lines.next_line().await.is_err());
    assert!(lines.next_line().await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    bounded(&lines);
}
