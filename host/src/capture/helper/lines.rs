//! Cancellation-safe helper announcement framing, bounded before accumulation.
use anyhow::{ensure, Result};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

/// Maximum complete wire record, including LF/CRLF when present.
const MAX_RECORD_BYTES: usize = 1024;

pub(super) struct Lines<R> {
    reader: BufReader<R>,
    bytes: Vec<u8>,
    failed: bool,
}

impl<R: AsyncRead + Unpin> Lines<R> {
    pub(super) fn new(reader: R) -> Self {
        Self {
            reader: BufReader::with_capacity(MAX_RECORD_BYTES, reader),
            bytes: Vec::with_capacity(MAX_RECORD_BYTES),
            failed: false,
        }
    }

    pub(super) async fn next_line(&mut self) -> Result<Option<String>> {
        ensure!(!self.failed, "helper announcement stream failed");
        let result = self.read_line().await;
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    async fn read_line(&mut self) -> Result<Option<String>> {
        loop {
            let available = self.reader.fill_buf().await?;
            if available.is_empty() {
                return self.finish();
            }
            let newline = available.iter().position(|&byte| byte == b'\n');
            let length = newline.map_or(available.len(), |index| index + 1);
            ensure!(
                length <= MAX_RECORD_BYTES - self.bytes.len(),
                "helper announcement exceeds 1024 bytes"
            );
            self.bytes.extend_from_slice(&available[..length]);
            self.reader.consume(length);
            if newline.is_some() {
                return self.finish();
            }
        }
    }

    fn finish(&mut self) -> Result<Option<String>> {
        if self.bytes.is_empty() {
            return Ok(None);
        }
        let bytes = match self.bytes.strip_suffix(b"\n") {
            Some(line) => line.strip_suffix(b"\r").unwrap_or(line),
            None => &self.bytes,
        };
        let line = std::str::from_utf8(bytes)?.to_owned();
        self.bytes.clear();
        Ok(Some(line))
    }
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
